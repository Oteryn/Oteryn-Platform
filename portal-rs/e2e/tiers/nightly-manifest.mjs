import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { discoveredCases, planNightly, requireNightlyExecution, requireNightlyPlan } from './nightly-plan.mjs';

const [command, directory, executions, publishedProjects] = process.argv.slice(2);
const cwd = fileURLToPath(new URL('../', import.meta.url));
const readJson = async path => JSON.parse(await readFile(path, 'utf8'));

function projectNames(names) {
  if (!Array.isArray(names) || !names.length ||
      names.some(name => typeof name !== 'string' || !/^[a-zA-Z0-9][a-zA-Z0-9._-]*$/.test(name)) ||
      new Set(names).size !== names.length) {
    throw new Error('Playwright projects must have unique names safe for artifact paths');
  }
  return names;
}

if (command === 'plan' && directory && !executions) {
  const { default: config } = await import('./playwright.config.mjs');
  const projects = projectNames(config.projects.map(project => project.name));
  const scratch = await mkdtemp(join(tmpdir(), 'portal-e2e-discovery-'));
  try {
    await mkdir(directory, { recursive: true });
    for (const project of projects) {
      const plan = await planNightly(project, async (filters, label) => {
        const report = join(scratch, `${project}-${label}.json`);
        try {
          execFileSync(process.execPath, [
            'node_modules/@playwright/test/cli.js', 'test', '--config=tiers/playwright.config.mjs',
            `--project=${project}`, ...filters, '--list', '--reporter=json',
          ], { cwd, env: { ...process.env, PLAYWRIGHT_JSON_OUTPUT_FILE: report }, stdio: 'ignore' });
        } catch {
          throw new Error(`Playwright discovery failed for ${project} (${label})`);
        }
        return discoveredCases((await readJson(report)).suites);
      });
      await writeFile(join(directory, `${project}.json`), JSON.stringify(plan));
      console.error(`Planned ${plan.cases.length} cases for ${project}`);
    }
    console.log(JSON.stringify(projects));
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
} else if (command === 'verify' && directory && executions && publishedProjects) {
  for (const project of projectNames(JSON.parse(publishedProjects))) {
    const plan = requireNightlyPlan(await readJson(join(directory, `${project}.json`)), project);
    const reports = await Promise.all(plan.shards.map((_, index) =>
      readJson(join(executions, `nightly-cases-${project}-${index + 1}`, 'tier-execution.json'))));
    requireNightlyExecution(plan, reports);
    console.log(`Verified ${plan.cases.length} executed cases for ${project}`);
  }
} else {
  throw new Error('Usage: node tiers/nightly-manifest.mjs plan PLAN_DIR | verify PLAN_DIR EXECUTION_DIR PROJECTS_JSON');
}
