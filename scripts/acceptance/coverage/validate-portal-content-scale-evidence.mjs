import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(coverageRoot, '../../..');
const requiredViewports = ['desktop-1440x1000', 'tablet-820x1180', 'mobile-390x844'];
const requiredAssertions = ['readable_wrapping', 'component_containment', 'stable_bounded_pagination', 'no_document_horizontal_overflow'];
const classifications = ['long_content_consumer', 'large_collection_consumer', 'long_content_and_large_collection_consumer', 'not_applicable', 'supporting_endpoint'];

function readJson(file) { return JSON.parse(fs.readFileSync(file, 'utf8')); }
function nonEmpty(value) { return typeof value === 'string' && value.trim() !== ''; }
function stringArray(value) { return Array.isArray(value) && value.length > 0 && value.every(nonEmpty); }
function sameSet(actual, expected) {
  return Array.isArray(actual)
    && actual.length === expected.length
    && JSON.stringify([...actual].sort()) === JSON.stringify([...expected].sort());
}
function expectedAssertions(classification) {
  if (classification === 'long_content_consumer') return ['readable_wrapping', 'component_containment', 'no_document_horizontal_overflow'];
  if (classification === 'large_collection_consumer') return ['component_containment', 'stable_bounded_pagination', 'no_document_horizontal_overflow'];
  if (classification === 'long_content_and_large_collection_consumer') return requiredAssertions;
  return [];
}
function isConsumer(classification) { return classification.endsWith('_consumer'); }
function safeFile(repoRoot, owner, relative, errors) {
  if (!nonEmpty(relative)) { errors.push(`${owner} must define a repository-relative file.`); return null; }
  const absolute = path.resolve(repoRoot, relative);
  if (!absolute.startsWith(`${repoRoot}${path.sep}`)) { errors.push(`${owner} escapes repository root: ${relative}`); return null; }
  if (!fs.existsSync(absolute) || !fs.statSync(absolute).isFile()) { errors.push(`${owner} references missing file ${relative}.`); return null; }
  return absolute;
}

function loadPortalManifestSurfaces(repoRoot) {
  const manifest = readJson(path.join(repoRoot, 'scripts/acceptance/coverage/portal-coverage-manifest.json'));
  const surfaces = Array.isArray(manifest.surfaces) ? [...manifest.surfaces] : [];
  const fragmentRoot = path.join(repoRoot, 'scripts/acceptance/coverage/surfaces');
  if (!fs.existsSync(fragmentRoot)) return surfaces;

  for (const name of fs.readdirSync(fragmentRoot).filter((entry) => entry.endsWith('.json')).sort()) {
    const fragment = readJson(path.join(fragmentRoot, name));
    const fragmentSurfaces = Array.isArray(fragment) ? fragment : fragment?.surfaces;
    if (!Array.isArray(fragmentSurfaces) || fragmentSurfaces.length === 0) {
      throw new Error(`Portal coverage fragment ${name} must define at least one surface.`);
    }
    surfaces.push(...fragmentSurfaces);
  }

  return surfaces;
}

function loadSurfaceClassifications(repoRoot, baseRecords) {
  const records = { ...(baseRecords ?? {}) };
  const fragmentRoot = path.join(repoRoot, 'docs/testing/portal-content-scale-surfaces');
  if (!fs.existsSync(fragmentRoot)) return records;

  for (const name of fs.readdirSync(fragmentRoot).filter((entry) => entry.endsWith('.json')).sort()) {
    const fragment = readJson(path.join(fragmentRoot, name));
    const fragmentRecords = fragment?.surfaces;
    if (!fragmentRecords || typeof fragmentRecords !== 'object' || Array.isArray(fragmentRecords)) {
      throw new Error(`Content scale fragment ${name} must define a surfaces object.`);
    }
    for (const [id, record] of Object.entries(fragmentRecords)) {
      if (Object.prototype.hasOwnProperty.call(records, id)) {
        throw new Error(`Duplicate content scale classification across base and fragments: ${id}`);
      }
      records[id] = record;
    }
  }

  return records;
}

function validateProfile(repoRoot, id, profile, packageScripts, errors) {
  const owner = `content scale profile ${id}`;
  if (!nonEmpty(profile?.npm_profile) || !nonEmpty(packageScripts?.[profile.npm_profile])) errors.push(`${owner} references unknown npm profile ${JSON.stringify(profile?.npm_profile)}.`);
  const config = safeFile(repoRoot, `${owner} config`, profile?.config_file, errors);
  const workflow = safeFile(repoRoot, `${owner} workflow`, profile?.workflow_file, errors);
  const configSource = config ? fs.readFileSync(config, 'utf8') : '';
  const workflowSource = workflow ? fs.readFileSync(workflow, 'utf8') : '';
  if (!stringArray(profile?.projects) || profile.projects.length !== 3) errors.push(`${owner} must define exactly three Playwright projects.`);
  else for (const project of profile.projects) {
    if (!configSource.includes(`name: '${project}'`) && !configSource.includes(`name: "${project}"`)) errors.push(`${owner} project ${project} is not declared in ${profile.config_file}.`);
  }
  if (!sameSet(profile?.viewports, requiredViewports)) errors.push(`${owner} must declare the exact required viewport set.`);
  if (profile?.retries !== 0 || !configSource.includes('retries: 0')) errors.push(`${owner} must remain zero retry in contract and config.`);
  if (!nonEmpty(profile?.workflow_marker) || !workflowSource.includes(profile.workflow_marker)) errors.push(`${owner} workflow marker is missing from ${profile?.workflow_file}.`);
  const command = packageScripts?.[profile?.npm_profile] ?? '';
  if (nonEmpty(profile?.config_file) && !command.includes(path.basename(profile.config_file))) errors.push(`${owner} npm profile does not execute ${profile.config_file}.`);
  if (!nonEmpty(profile?.runtime)) errors.push(`${owner} must describe the real runtime boundary.`);
  if (!/^[0-9a-f]{40}$/u.test(profile?.validated_sha ?? '')) errors.push(`${owner} must record a full validated SHA.`);
  if (!Number.isInteger(profile?.validated_workflow_run_id) || profile.validated_workflow_run_id < 1) errors.push(`${owner} must record a positive workflow run id.`);
}

function validateEvidenceGroup(repoRoot, id, group, errors) {
  const owner = `content scale evidence group ${id}`;
  safeFile(repoRoot, `${owner} fixture`, group?.fixture_file, errors);
  const evidence = safeFile(repoRoot, `${owner} evidence`, group?.evidence_file, errors);
  if (!nonEmpty(group?.marker)) errors.push(`${owner} must define a stable marker.`);
  else if (evidence && !fs.readFileSync(evidence, 'utf8').includes(group.marker)) errors.push(`${owner} marker is missing from ${group.evidence_file}: ${group.marker}`);
  if (!stringArray(group?.routes)) errors.push(`${owner} must define at least one real route.`);
  if ('bounded_rows' in (group ?? {}) && (!Number.isInteger(group.bounded_rows) || group.bounded_rows < 2)) errors.push(`${owner} bounded_rows must be an integer of at least 2.`);
}

export function validatePortalContentScaleEvidence({ contract, manifestSurfaces, packageScripts, repoRoot = defaultRepoRoot }) {
  const errors = [];
  const warnings = [];
  if (contract?.schema_version !== 2) errors.push('portal content scale schema_version must be 2.');
  if (contract?.status !== 'complete') errors.push('portal content scale status must be complete.');
  if (!sameSet(contract?.classification_values, classifications)) errors.push('portal content scale classification_values do not match the final contract values.');
  if (!Array.isArray(contract?.nonclaims) || contract.nonclaims.length < 3 || !contract.nonclaims.every(nonEmpty)) errors.push('portal content scale evidence must preserve at least three explicit nonclaims.');

  const evidence = contract?.evidence_contract ?? {};
  if (evidence.status !== 'complete') errors.push('evidence_contract status must be complete.');
  if (!sameSet(evidence.required_viewports, requiredViewports)) errors.push('required_viewports must match desktop, tablet and mobile.');
  if (!sameSet(evidence.required_assertions, requiredAssertions)) errors.push('required_assertions must match the exact scale contract.');

  const manifest = new Map();
  for (const surface of Array.isArray(manifestSurfaces) ? manifestSurfaces : []) {
    if (!nonEmpty(surface?.id)) { errors.push('Every portal manifest surface must have a stable id.'); continue; }
    if (manifest.has(surface.id)) errors.push(`Duplicate portal manifest surface id: ${surface.id}`);
    manifest.set(surface.id, surface);
  }
  const records = contract?.surfaces && typeof contract.surfaces === 'object' ? contract.surfaces : {};
  for (const id of manifest.keys()) if (!(id in records)) errors.push(`Missing content scale classification for portal surface: ${id}`);
  for (const id of Object.keys(records)) if (!manifest.has(id)) errors.push(`Content scale classification references unknown portal surface: ${id}`);

  const consumers = [];
  for (const [id, record] of Object.entries(records)) {
    const classification = record?.classification;
    if (!classifications.includes(classification)) { errors.push(`${id} has unsupported classification ${JSON.stringify(classification)}.`); continue; }
    if (!nonEmpty(record?.rationale) || record.rationale.trim().length < 40) errors.push(`${id} requires a bounded rationale of at least 40 characters.`);
    const manifestSurface = manifest.get(id);
    if (manifestSurface?.status === 'supporting_endpoint' && classification !== 'supporting_endpoint') errors.push(`${id} must remain supporting_endpoint.`);
    if (manifestSurface?.status !== 'supporting_endpoint' && classification === 'supporting_endpoint') errors.push(`${id} cannot be supporting_endpoint.`);
    if (isConsumer(classification)) consumers.push(id);
  }

  const profiles = evidence.profiles && typeof evidence.profiles === 'object' ? evidence.profiles : {};
  const groups = evidence.evidence_groups && typeof evidence.evidence_groups === 'object' ? evidence.evidence_groups : {};
  const mapped = evidence.mapped_surfaces && typeof evidence.mapped_surfaces === 'object' ? evidence.mapped_surfaces : {};
  const gaps = evidence.gap_surfaces && typeof evidence.gap_surfaces === 'object' ? evidence.gap_surfaces : {};
  if (Object.keys(gaps).length !== 0) errors.push(`complete content scale closure requires zero gap surfaces; found ${Object.keys(gaps).length}.`);

  for (const [id, profile] of Object.entries(profiles)) validateProfile(repoRoot, id, profile, packageScripts, errors);
  for (const [id, group] of Object.entries(groups)) validateEvidenceGroup(repoRoot, id, group, errors);
  for (const id of Object.keys(mapped)) if (!consumers.includes(id)) errors.push(`Orphan content scale mapping references non-consumer surface: ${id}`);

  const usedProfiles = new Set();
  const usedGroups = new Set();
  for (const id of consumers) {
    const mapping = mapped[id];
    if (!mapping) { errors.push(`Missing executable content scale mapping for consumer surface: ${id}`); continue; }
    if (!nonEmpty(mapping.profile) || !(mapping.profile in profiles)) errors.push(`${id} references unknown content scale profile ${JSON.stringify(mapping.profile)}.`);
    else usedProfiles.add(mapping.profile);
    if (!stringArray(mapping.evidence_groups)) errors.push(`${id} must reference at least one evidence group.`);
    else for (const groupId of mapping.evidence_groups) {
      if (!(groupId in groups)) errors.push(`${id} references unknown evidence group ${groupId}.`);
      else usedGroups.add(groupId);
    }
    const expected = expectedAssertions(records[id].classification);
    if (!sameSet(mapping.assertions, expected)) errors.push(`${id} assertions must exactly match ${JSON.stringify(expected)}.`);
    if (!stringArray(mapping.proven_assertions)) errors.push(`${id} must define concrete proven assertions.`);
    if (!Array.isArray(mapping.remaining_gaps) || mapping.remaining_gaps.length !== 0) errors.push(`${id} must have zero remaining_gaps.`);
    if (expected.includes('stable_bounded_pagination')) {
      const hasBoundedGroup = Array.isArray(mapping.evidence_groups) && mapping.evidence_groups.some((groupId) => Number.isInteger(groups[groupId]?.bounded_rows) && groups[groupId].bounded_rows >= 2);
      if (!hasBoundedGroup) errors.push(`${id} requires at least one bounded large-collection evidence group.`);
    }
  }
  for (const id of Object.keys(profiles)) if (!usedProfiles.has(id)) errors.push(`Orphan content scale profile is not referenced: ${id}`);
  for (const id of Object.keys(groups)) if (!usedGroups.has(id)) errors.push(`Orphan content scale evidence group is not referenced: ${id}`);

  return {
    schema_version: contract?.schema_version ?? null,
    status: contract?.status ?? null,
    portal_surface_count: manifest.size,
    classified_surface_count: Object.keys(records).length,
    consumer_surface_count: consumers.length,
    mapped_surface_count: Object.keys(mapped).length,
    profile_count: Object.keys(profiles).length,
    evidence_group_count: Object.keys(groups).length,
    gap_surface_count: Object.keys(gaps).length,
    errors,
    warnings,
  };
}

export function loadRepositoryInputs(repoRoot = defaultRepoRoot) {
  const packageJson = readJson(path.join(repoRoot, 'scripts/acceptance/package.json'));
  const contract = readJson(path.join(repoRoot, 'docs/testing/PORTAL_CONTENT_SCALE_EVIDENCE.json'));
  contract.surfaces = loadSurfaceClassifications(repoRoot, contract.surfaces);
  return {
    contract,
    manifestSurfaces: loadPortalManifestSurfaces(repoRoot),
    packageScripts: packageJson.scripts ?? {},
    repoRoot,
  };
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  let report;
  try { report = validatePortalContentScaleEvidence(loadRepositoryInputs()); }
  catch (error) {
    report = { schema_version: null, status: null, portal_surface_count: 0, classified_surface_count: 0, consumer_surface_count: 0, mapped_surface_count: 0, profile_count: 0, evidence_group_count: 0, gap_surface_count: 0, errors: [`Cannot validate portal content scale evidence: ${error.message}`], warnings: [] };
  }
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.errors.length > 0) process.exitCode = 1;
}
