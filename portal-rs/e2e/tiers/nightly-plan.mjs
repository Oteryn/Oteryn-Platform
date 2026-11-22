import { basename } from 'node:path';

export function discoveredCases(suites) {
  const cases = [];
  function visit(suite) {
    for (const spec of suite.specs ?? []) {
      for (const test of spec.tests) {
        cases.push(JSON.stringify([spec.file, spec.id, test.projectId]));
      }
    }
    for (const child of suite.suites ?? []) visit(child);
  }
  suites.forEach(visit);
  return cases.sort();
}

export async function planNightly(discover) {
  const all = await discover([], 'all');
  const files = [...new Set(all.map(test => JSON.parse(test)[0]))].sort();
  // The staff bootstrap and its consumers must share browser storage state.
  const usesStaffState = file => /^(?:admin\.|content\.|game-sim-bazaar\.|routes\.|staff-)/.test(basename(file));
  const remaining = files.filter(file => !usesStaffState(file));
  const groups = [files.filter(usesStaffState), ...[0, 1].map(part => remaining.filter((_, i) => i % 2 === part))];
  const plans = await Promise.all(groups.map(async (group, i) => {
    if (!group.length) throw new Error(`Empty nightly shard ${i + 1}`);
    const filters = group.map(file => '/' + file.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '$');
    return { filters, cases: await discover(filters, `shard-${i + 1}`) };
  }));
  const combined = plans.flatMap(plan => plan.cases).sort();
  if (new Set(all).size !== all.length || plans.some(plan => !plan.cases.length) ||
      JSON.stringify(combined) !== JSON.stringify(all)) {
    throw new Error('Nightly shards must cover every discovered Playwright case exactly once');
  }
  return plans;
}
