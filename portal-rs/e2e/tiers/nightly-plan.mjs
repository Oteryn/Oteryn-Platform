import { basename } from 'node:path';

export function* playwrightCases(suites) {
  for (const suite of suites) {
    for (const spec of suite.specs ?? []) {
      for (const test of spec.tests) yield { spec, test };
    }
    yield* playwrightCases(suite.suites ?? []);
  }
}

export function discoveredCases(suites) {
  return Array.from(playwrightCases(suites), ({ spec, test }) =>
    JSON.stringify([spec.file, spec.id, test.projectId])).sort();
}

function requireSameCases(actual, expected, message) {
  if (JSON.stringify([...actual].sort()) !== JSON.stringify([...expected].sort())) {
    throw new Error(message);
  }
}

export function requireNightlyPlan(plan, project) {
  if (plan.project !== project || plan.shards.length !== 3 ||
      new Set(plan.cases).size !== plan.cases.length ||
      plan.shards.some(shard => !shard.cases.length || !shard.filters.length)) {
    throw new Error(`Invalid nightly plan for ${project}`);
  }
  requireSameCases(plan.shards.flatMap(shard => shard.cases), plan.cases,
    `Nightly shards must cover every discovered Playwright case exactly once for ${project}`);
  return plan;
}

export function requireNightlyExecution(plan, reports) {
  requireNightlyPlan(plan, plan.project);
  if (reports.length !== plan.shards.length) throw new Error(`Missing execution reports for ${plan.project}`);
  const executed = reports.flatMap((report, index) => {
    const shard = index + 1;
    if (report.project !== plan.project || report.shard !== shard) {
      throw new Error(`Execution report does not identify ${plan.project} shard ${shard}`);
    }
    requireSameCases(report.cases, plan.shards[index].cases,
      `Execution differs from the published plan for ${plan.project} shard ${shard}`);
    return report.cases;
  });
  requireSameCases(executed, plan.cases,
    `Nightly execution must cover every discovered Playwright case exactly once for ${plan.project}`);
}

export async function planNightly(project, discover) {
  const all = await discover([], 'all');
  const files = [...new Set(all.map(test => JSON.parse(test)[0]))].sort();
  // The staff bootstrap and its consumers must share browser storage state.
  const usesStaffState = file => /^(?:admin\.|content\.|game-sim-bazaar\.|routes\.|staff-)/.test(basename(file));
  const remaining = files.filter(file => !usesStaffState(file));
  const groups = [files.filter(usesStaffState), ...[0, 1].map(part => remaining.filter((_, i) => i % 2 === part))];
  const shards = await Promise.all(groups.map(async (group, i) => {
    if (!group.length) throw new Error(`Empty nightly shard ${i + 1}`);
    const filters = group.map(file => '/' + file.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '$');
    return { filters, cases: await discover(filters, `shard-${i + 1}`) };
  }));
  return requireNightlyPlan({ project, cases: all, shards }, project);
}
