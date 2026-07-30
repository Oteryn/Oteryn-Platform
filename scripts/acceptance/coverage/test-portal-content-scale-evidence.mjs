import assert from 'node:assert/strict';
import {
  loadRepositoryInputs,
  validatePortalContentScaleEvidence,
} from './validate-portal-content-scale-evidence.mjs';

function clone(value) {
  return structuredClone(value);
}

function expectError(inputs, mutation, expectedMarker) {
  const candidate = clone(inputs);
  mutation(candidate);
  const report = validatePortalContentScaleEvidence(candidate);
  assert.ok(
    report.errors.some((error) => error.includes(expectedMarker)),
    `Expected error containing ${JSON.stringify(expectedMarker)}, received:\n${report.errors.join('\n')}`,
  );
}

const inputs = loadRepositoryInputs();
const baseline = validatePortalContentScaleEvidence(inputs);
assert.deepEqual(baseline.errors, [], `Repository content-scale ledger is invalid:\n${baseline.errors.join('\n')}`);
assert.equal(baseline.portal_surface_count, 18);
assert.equal(baseline.classified_surface_count, 18);
assert.equal(baseline.candidate_surface_count, 12);
assert.equal(baseline.mapped_surface_count, 2);
assert.equal(baseline.gap_surface_count, 10);

expectError(inputs, (candidate) => {
  delete candidate.contract.surfaces['public.home-and-seo'];
}, 'Missing content scale classification for portal surface: public.home-and-seo');

expectError(inputs, (candidate) => {
  candidate.contract.surfaces['unknown.portal.surface'] = {
    classification: 'not_applicable',
    rationale: 'A deliberately invalid unknown surface used by the deterministic negative fixture.',
  };
}, 'Content scale classification references unknown portal surface: unknown.portal.surface');

expectError(inputs, (candidate) => {
  delete candidate.contract.evidence_contract.gap_surfaces['public.home-and-seo'];
}, 'public.home-and-seo must be classified exactly once as mapped or gap; found 0');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.gap_surfaces['public.news-and-managed-pages'] = {
    reason: 'This deliberately duplicates an executable mapping and must be rejected by the fail-closed contract validator.',
    missing: ['readable_wrapping', 'component_containment', 'no_document_horizontal_overflow'],
  };
}, 'public.news-and-managed-pages cannot be both mapped and an explicit gap');

expectError(inputs, (candidate) => {
  candidate.contract.surfaces['account.overview-provisioning'].rationale = 'short';
}, 'account.overview-provisioning requires a bounded classification rationale');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['public.news-and-managed-pages'].fixture.evidence_file = 'scripts/acceptance/tests/missing-content-scale.spec.mjs';
}, 'references missing file scripts/acceptance/tests/missing-content-scale.spec.mjs');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['public.news-and-managed-pages'].fixture.marker = '@missing-content-scale-marker';
}, 'evidence marker is missing');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['public.news-and-managed-pages'].execution.npm_profile = 'test:unknown-content-scale';
}, 'references an unknown npm profile');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['public.news-and-managed-pages'].execution.projects[0] = 'unknown-content-scale-project';
}, 'project unknown-content-scale-project is not declared');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['public.news-and-managed-pages'].assertions = [
    'readable_wrapping',
    'no_document_horizontal_overflow',
  ];
}, 'assertions must exactly match');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['identity.registration-login-session'] = clone(
    candidate.contract.evidence_contract.mapped_surfaces['public.news-and-managed-pages'],
  );
}, 'Orphan content scale mapping references non-candidate surface: identity.registration-login-session');

expectError(inputs, (candidate) => {
  candidate.contract.status = 'complete';
  candidate.contract.evidence_contract.status = 'complete';
}, 'complete content scale closure requires zero gap surfaces');

process.stdout.write(`${JSON.stringify({
  baseline_surfaces: baseline.classified_surface_count,
  baseline_candidates: baseline.candidate_surface_count,
  baseline_mapped: baseline.mapped_surface_count,
  baseline_gaps: baseline.gap_surface_count,
  negative_fixtures: 12,
  result: 'PASS',
}, null, 2)}\n`);
