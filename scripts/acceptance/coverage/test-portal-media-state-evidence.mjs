import assert from 'node:assert/strict';
import {
  loadRepositoryInputs,
  validatePortalMediaStateEvidence,
} from './validate-portal-media-state-evidence.mjs';

function clone(value) {
  return structuredClone(value);
}

function expectFailure(name, mutate, expectedMarker) {
  const baseline = loadRepositoryInputs();
  const contract = clone(baseline.contract);
  const manifestSurfaces = clone(baseline.manifestSurfaces);
  const dimensionContract = clone(baseline.dimensionContract);
  const dimensionSurfaces = clone(baseline.dimensionSurfaces);

  mutate({ contract, manifestSurfaces, dimensionContract, dimensionSurfaces });

  const report = validatePortalMediaStateEvidence({
    contract,
    manifestSurfaces,
    dimensionContract,
    dimensionSurfaces,
    repoRoot: baseline.repoRoot,
  });
  assert.ok(
    report.errors.some((error) => error.includes(expectedMarker)),
    `${name} did not fail with ${JSON.stringify(expectedMarker)}.\n${JSON.stringify(report, null, 2)}`,
  );
}

const baseline = loadRepositoryInputs();
const baselineReport = validatePortalMediaStateEvidence(baseline);
assert.deepEqual(baselineReport.errors, [], JSON.stringify(baselineReport, null, 2));
assert.equal(baselineReport.gap_state_count, 0, JSON.stringify(baselineReport, null, 2));

expectFailure('missing portal classification', ({ contract }) => {
  contract.surfaces = contract.surfaces.filter((surface) => surface.id !== 'public.home-and-seo');
}, 'Missing media classification for portal surface');

expectFailure('orphan media classification', ({ contract }) => {
  contract.surfaces.push({
    id: 'orphan.media-surface',
    classification: 'not_applicable',
    rationale: 'This deliberately invalid record is not present in the canonical portal surface inventory and must fail closed.',
  });
}, 'references unknown portal surface');

expectFailure('supporting endpoint promoted to rendered consumer', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'browser-supporting-media-preview-endpoints');
  surface.classification = 'media_consumer';
  surface.states = {};
  delete surface.rationale;
}, 'must be classified as supporting_endpoint');

expectFailure('missing required rendered state', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'wiki.public');
  delete surface.states.missing;
}, 'rendered state set does not match required_rendered_states');

expectFailure('missing evidence marker', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'editorial-media.admin');
  surface.states.normal.evidence.markers = ['marker-that-does-not-exist'];
}, 'evidence marker not found');

expectFailure('unknown execution profile', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'wiki.public');
  surface.states.normal.evidence.profile = 'profile-that-does-not-exist';
}, 'references unknown execution profile');

expectFailure('unknown Playwright project', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'wiki.admin-editorial-lifecycle');
  surface.states.normal.evidence.projects = ['project-that-does-not-exist'];
}, 'references unknown Playwright project');

expectFailure('unbounded gap reason', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'editorial-media.admin');
  surface.states.missing = {
    status: 'gap',
    reason: 'missing',
  };
}, 'gap requires a bounded reason');

expectFailure('unbounded non-media rationale', ({ contract }) => {
  const surface = contract.surfaces.find((record) => record.id === 'public.game-data');
  surface.rationale = 'not media';
}, 'not_applicable classification requires a bounded rationale');

process.stdout.write(`${JSON.stringify({
  portal_surfaces: baselineReport.portal_surface_count,
  media_consumers: baselineReport.media_consumer_count,
  evidenced_states: baselineReport.evidenced_state_count,
  explicit_gaps: baselineReport.gap_state_count,
  negative_fixtures: 9,
  status: 'pass',
}, null, 2)}\n`);
