import assert from 'node:assert/strict';
import { loadBaseline, validatePortalEvidenceDimensions } from './validate-portal-evidence-dimensions.mjs';

function clone(value) {
  return structuredClone(value);
}

function expectFailure(name, mutate, expectedMarker) {
  const baseline = loadBaseline();
  const contract = clone(baseline.contract);
  const manifestSurfaces = clone(baseline.manifestSurfaces);
  mutate({ contract, manifestSurfaces });
  const report = validatePortalEvidenceDimensions({ contract, manifestSurfaces, repoRoot: baseline.repoRoot });
  assert.ok(
    report.errors.some((error) => error.includes(expectedMarker)),
    `${name} did not fail with ${JSON.stringify(expectedMarker)}.\n${JSON.stringify(report, null, 2)}`,
  );
}

const baseline = loadBaseline();
const baselineReport = validatePortalEvidenceDimensions(baseline);
assert.deepEqual(baselineReport.errors, [], JSON.stringify(baselineReport, null, 2));

expectFailure('missing mobile viewport evidence', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'public.home-and-seo');
  delete surface.viewports['mobile-390x844'];
}, 'viewport mappings do not match declared viewports');

expectFailure('unknown Playwright project', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'events.public-admin');
  surface.viewports['tablet-820x1180'].project = 'events-tablet-does-not-exist';
}, 'references unknown project');

expectFailure('missing portability rationale', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'identity.password-lifecycle');
  surface.portability.rationale = '';
}, 'portability exclusion requires a bounded risk-based rationale');

expectFailure('orphan dimension record', ({ contract }) => {
  contract.surfaces.push({
    id: 'orphan.surface',
    kind: 'supporting_endpoint',
    critical: false,
    rationale: 'This deliberately invalid fixture is not present in the portal manifest and must fail closed.',
  });
}, 'Dimension record references unknown surface');

expectFailure('missing exact evidence marker', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'public.game-data');
  const evidenceId = surface.viewports['desktop-1440x1000'].evidence;
  const evidence = surface.evidence.find((record) => record.id === evidenceId);
  evidence.markers = ['marker-that-does-not-exist'];
}, 'evidence marker not found');

expectFailure('unknown browser declaration', ({ manifestSurfaces }) => {
  const surface = manifestSurfaces.find((record) => record.id === 'public.home-and-seo');
  surface.browsers.push('browser-that-does-not-exist');
}, 'declares unknown browser/profile id');

process.stdout.write(`${JSON.stringify({
  baseline_surfaces: baselineReport.manifest_surface_count,
  baseline_profiles: baselineReport.execution_profile_count,
  baseline_critical_surfaces: baselineReport.critical_surface_count,
  negative_fixtures: 6,
  status: 'pass',
}, null, 2)}\n`);
