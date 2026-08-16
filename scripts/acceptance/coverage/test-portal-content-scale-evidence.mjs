import assert from 'node:assert/strict';
import {
  loadRepositoryInputs,
  validatePortalContentScaleEvidence,
} from './validate-portal-content-scale-evidence.mjs';

function clone(value) { return structuredClone(value); }
function expectError(inputs, mutation, marker) {
  const candidate = clone(inputs);
  mutation(candidate);
  const report = validatePortalContentScaleEvidence(candidate);
  assert.ok(
    report.errors.some((error) => error.includes(marker)),
    `Expected error containing ${JSON.stringify(marker)}, received:\n${report.errors.join('\n')}`,
  );
}

const inputs = loadRepositoryInputs();
const baseline = validatePortalContentScaleEvidence(inputs);
assert.deepEqual(baseline.errors, [], `Repository content-scale ledger is invalid:\n${baseline.errors.join('\n')}`);
assert.equal(baseline.schema_version, 2);
assert.equal(baseline.status, 'complete');
assert.equal(baseline.portal_surface_count, 31);
assert.equal(baseline.classified_surface_count, 31);
assert.equal(baseline.consumer_surface_count, 12);
assert.equal(baseline.mapped_surface_count, 12);
assert.equal(baseline.profile_count, 2);
assert.equal(baseline.evidence_group_count, 6);
assert.equal(baseline.gap_surface_count, 0);

expectError(inputs, (candidate) => {
  delete candidate.contract.surfaces['public.home-and-seo'];
}, 'Missing content scale classification for portal surface: public.home-and-seo');

expectError(inputs, (candidate) => {
  candidate.manifestSurfaces.push({ id: 'fragment.future-surface', status: 'covered' });
}, 'Missing content scale classification for portal surface: fragment.future-surface');

expectError(inputs, (candidate) => {
  candidate.contract.surfaces['unknown.portal.surface'] = {
    classification: 'not_applicable',
    rationale: 'A deliberately invalid unknown surface used by the deterministic negative fixture.',
  };
}, 'Content scale classification references unknown portal surface: unknown.portal.surface');

expectError(inputs, (candidate) => {
  delete candidate.contract.evidence_contract.mapped_surfaces['public.home-and-seo'];
}, 'Missing executable content scale mapping for consumer surface: public.home-and-seo');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['identity.registration-login-session'] = clone(
    candidate.contract.evidence_contract.mapped_surfaces['public.home-and-seo'],
  );
}, 'Orphan content scale mapping references non-consumer surface: identity.registration-login-session');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.evidence_groups['public-content'].evidence_file = 'scripts/acceptance/tests/missing-content-scale.spec.mjs';
}, 'references missing file scripts/acceptance/tests/missing-content-scale.spec.mjs');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.evidence_groups['public-content'].marker = '@missing-content-scale-marker';
}, 'marker is missing');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['public.home-and-seo'].profile = 'unknown-profile';
}, 'references unknown content scale profile');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['public.home-and-seo'].evidence_groups = ['unknown-group'];
}, 'references unknown evidence group unknown-group');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['public.home-and-seo'].assertions = [
    'component_containment',
    'no_document_horizontal_overflow',
  ];
}, 'assertions must exactly match');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.mapped_surfaces['editorial-media.admin'].evidence_groups = ['public-content'];
}, 'requires at least one bounded large-collection evidence group');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.profiles['orphan-profile'] = clone(candidate.contract.evidence_contract.profiles['content-scale']);
}, 'Orphan content scale profile is not referenced: orphan-profile');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.evidence_groups['orphan-group'] = clone(candidate.contract.evidence_contract.evidence_groups.media);
}, 'Orphan content scale evidence group is not referenced: orphan-group');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.gap_surfaces['public.home-and-seo'] = {
    reason: 'A complete contract cannot preserve an executable evidence gap.',
  };
}, 'complete content scale closure requires zero gap surfaces');

expectError(inputs, (candidate) => {
  candidate.contract.evidence_contract.profiles['content-scale'].validated_sha = 'not-a-sha';
}, 'must record a full validated SHA');

process.stdout.write(`${JSON.stringify({
  baseline_surfaces: baseline.classified_surface_count,
  baseline_consumers: baseline.consumer_surface_count,
  baseline_mapped: baseline.mapped_surface_count,
  baseline_profiles: baseline.profile_count,
  baseline_evidence_groups: baseline.evidence_group_count,
  baseline_gaps: baseline.gap_surface_count,
  negative_fixtures: 15,
  result: 'PASS',
}, null, 2)}\n`);
