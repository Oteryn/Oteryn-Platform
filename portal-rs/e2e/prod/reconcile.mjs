import { appendFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

const outcomes = ['success', 'failure', 'timed_out'];

export async function latestProbe(readAttempt, runs) {
  for (const { id, run_attempt: latest } of runs) {
    for (let attempt = latest; attempt > 0; attempt--) {
      const { run, jobs } = await readAttempt(id, attempt);
      if (run.id !== id || run.run_attempt !== attempt ||
          jobs.some(job => job.run_id !== id || job.run_attempt !== attempt)) {
        throw new Error('Actions returned an unexpected monitor attempt');
      }
      if (!jobs.length && run.status === 'completed' && run.conclusion === 'cancelled') continue;

      const probes = jobs.filter(job => job.name === 'monitor');
      if (probes.length > 1) throw new Error('Actions returned duplicate monitor jobs');
      const [probe] = probes;
      if (probe?.status === 'completed') {
        if (['skipped', 'cancelled'].includes(probe.conclusion)) continue;
        const measured = probe.steps?.find(step => ['Run production monitor', 'Run public monitor'].includes(step.name)
          && outcomes.includes(step.conclusion));
        if (measured) return { runId: id, attempt, conclusion: measured.conclusion };
        if (outcomes.includes(probe.conclusion)) return { runId: id, attempt, conclusion: probe.conclusion };
      }
      throw new Error('The previous monitor attempt is missing a terminal probe outcome');
    }
  }
  return null;
}

async function reconcile() {
  const { GITHUB_REPOSITORY, GITHUB_RUN_ID, GITHUB_RUN_ATTEMPT, GITHUB_REF_NAME, GITHUB_STEP_SUMMARY } = process.env;
  if (GITHUB_REPOSITORY !== 'Oteryn/Oteryn-Platform' || !GITHUB_STEP_SUMMARY) {
    throw new Error('Production history requires the Platform workflow');
  }
  // Public Actions history does not require a token with Actions permissions.
  const api = 'https://api.github.com/repos/Oteryn/Oteryn-Platform/actions/';
  let remainingReads = 24;
  const read = async path => {
    if (remainingReads-- <= 0) throw new Error('Monitor history read budget exhausted');
    const terms = { redirect: 'error', signal: AbortSignal.timeout(10_000) };
    const result = await fetch(new URL(path, api), terms);
    if (!result.ok) throw new Error('Could not read public Actions history');
    return result.json();
  };
  const currentId = Number(GITHUB_RUN_ID);
  const currentAttempt = Number(GITHUB_RUN_ATTEMPT);
  if (!Number.isSafeInteger(currentId) || currentId < 1 ||
      !Number.isSafeInteger(currentAttempt) || currentAttempt < 1) {
    throw new Error('The workflow did not provide a valid run and attempt');
  }
  const params = new URLSearchParams({ branch: GITHUB_REF_NAME, per_page: '10' });
  const { workflow_runs } = await read(`workflows/prod-synthetic-monitor.yml/runs?${params}`);
  const runs = [];
  if (currentAttempt > 1) runs.push({ id: currentId, run_attempt: currentAttempt - 1 });
  const current = await read(`runs/${currentId}/attempts/${currentAttempt}`);
  if (current.id !== currentId || current.run_attempt !== currentAttempt) {
    throw new Error('Actions returned an unexpected current monitor attempt');
  }
  for (const run of workflow_runs) {
    if (run.run_number < current.run_number) runs.push(run);
  }

  const previous = await latestProbe(async (id, attempt) => {
    const path = `runs/${id}/attempts/${attempt}`;
    const [run, listing] = await Promise.all([read(path), read(`${path}/jobs?per_page=100`)]);
    if (listing.total_count !== listing.jobs.length) throw new Error('Incomplete monitor job listing');
    return { run, jobs: listing.jobs };
  }, runs);
  const summary = previous
    ? `Previous monitor probe: **${previous.conclusion}** (run ${previous.runId}, attempt ${previous.attempt}).`
    : 'No earlier production probe was found in the recent workflow history.';
  await appendFile(GITHUB_STEP_SUMMARY, `${summary}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    await reconcile();
  } catch {
    console.error('Could not reconcile recent production monitor history.');
    process.exitCode = 1;
  }
}
