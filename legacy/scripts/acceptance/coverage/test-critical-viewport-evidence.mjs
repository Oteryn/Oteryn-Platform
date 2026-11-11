import assert from 'node:assert/strict';
import { loadBaseline } from './validate-portal-evidence-dimensions.mjs';
import { validateCriticalViewportEvidence } from './validate-critical-viewport-evidence.mjs';

const baseline = loadBaseline();
const valid = validateCriticalViewportEvidence(baseline);
assert.deepEqual(valid.errors, [], JSON.stringify(valid, null, 2));

const missingTablet = structuredClone(baseline);
const responsive = missingTablet.contract.profiles.find((profile) => profile.id === 'standard-responsive');
responsive.projects = responsive.projects.filter((project) => project.viewport !== 'tablet-820x1180');
const missingTabletReport = validateCriticalViewportEvidence(missingTablet);
assert.ok(
  missingTabletReport.errors.some((error) => error.includes('critical viewport tablet-820x1180')),
  JSON.stringify(missingTabletReport, null, 2),
);

const nonBlocking = structuredClone(baseline);
nonBlocking.contract.profiles.find((profile) => profile.id === 'standard-responsive').blocking = false;
const nonBlockingReport = validateCriticalViewportEvidence(nonBlocking);
assert.ok(
  nonBlockingReport.errors.some((error) => error.includes('blocking zero-retry Chromium project') || error.includes('blocking project-selected evidence mapping')),
  JSON.stringify(nonBlockingReport, null, 2),
);

process.stdout.write(`${JSON.stringify({
  baseline_critical_surfaces: valid.critical_surface_count,
  derived_mappings: valid.derived_mapping_count,
  negative_fixtures: 2,
  status: 'pass',
}, null, 2)}\n`);
