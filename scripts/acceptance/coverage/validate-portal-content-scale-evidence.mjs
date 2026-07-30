import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(coverageRoot, '../../..');
const requiredViewports = [
  'desktop-1440x1000',
  'tablet-820x1180',
  'mobile-390x844',
];
const requiredAssertions = [
  'readable_wrapping',
  'component_containment',
  'stable_bounded_pagination',
  'no_document_horizontal_overflow',
];
const classificationValues = [
  'candidate_long_content',
  'candidate_large_collection',
  'candidate_both',
  'not_applicable',
  'supporting_endpoint',
];

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() !== '';
}

function stringArray(value) {
  return Array.isArray(value) && value.length > 0 && value.every(nonEmptyString);
}

function sameStringSet(actual, expected) {
  return Array.isArray(actual)
    && actual.length === expected.length
    && JSON.stringify([...actual].sort()) === JSON.stringify([...expected].sort());
}

function expectedAssertions(classification) {
  if (classification === 'candidate_long_content') {
    return ['readable_wrapping', 'component_containment', 'no_document_horizontal_overflow'];
  }
  if (classification === 'candidate_large_collection') {
    return ['component_containment', 'stable_bounded_pagination', 'no_document_horizontal_overflow'];
  }
  if (classification === 'candidate_both') return requiredAssertions;
  return [];
}

function safeFile(repoRoot, owner, relativeFile, errors) {
  if (!nonEmptyString(relativeFile)) {
    errors.push(`${owner} must define a repository-relative file.`);
    return null;
  }

  const absolute = path.resolve(repoRoot, relativeFile);
  if (!absolute.startsWith(`${repoRoot}${path.sep}`)) {
    errors.push(`${owner} file escapes repository root: ${relativeFile}`);
    return null;
  }
  if (!fs.existsSync(absolute) || !fs.statSync(absolute).isFile()) {
    errors.push(`${owner} references missing file ${relativeFile}.`);
    return null;
  }

  return absolute;
}

function validateMapping(repoRoot, id, classification, mapping, packageScripts, errors) {
  const owner = `content scale mapping ${id}`;
  const expected = expectedAssertions(classification);

  const fixtureFile = safeFile(repoRoot, `${owner} fixture`, mapping?.fixture?.file, errors);
  const evidenceRelativeFile = mapping?.fixture?.evidence_file ?? mapping?.fixture?.file;
  const evidenceFile = safeFile(repoRoot, `${owner} evidence`, evidenceRelativeFile, errors);

  if (!nonEmptyString(mapping?.fixture?.marker)) {
    errors.push(`${owner} must define a stable evidence marker.`);
  } else if (evidenceFile !== null && !fs.readFileSync(evidenceFile, 'utf8').includes(mapping.fixture.marker)) {
    errors.push(`${owner} evidence marker is missing from ${evidenceRelativeFile}: ${mapping.fixture.marker}`);
  }

  if (!stringArray(mapping?.fixture?.routes)) {
    errors.push(`${owner} must define at least one real rendered route.`);
  }

  if (classification !== 'candidate_long_content') {
    if (!Number.isInteger(mapping?.fixture?.bounded_rows) || mapping.fixture.bounded_rows < 2) {
      errors.push(`${owner} large-collection fixture must define bounded_rows of at least 2.`);
    }
  }

  const execution = mapping?.execution;
  if (!nonEmptyString(execution?.npm_profile) || !nonEmptyString(packageScripts?.[execution.npm_profile])) {
    errors.push(`${owner} references an unknown npm profile ${JSON.stringify(execution?.npm_profile)}.`);
  }

  const configFile = safeFile(repoRoot, `${owner} config`, execution?.config_file, errors);
  const workflowFile = safeFile(repoRoot, `${owner} workflow`, execution?.workflow_file, errors);
  const configSource = configFile === null ? '' : fs.readFileSync(configFile, 'utf8');
  const workflowSource = workflowFile === null ? '' : fs.readFileSync(workflowFile, 'utf8');

  if (!stringArray(execution?.projects) || execution.projects.length !== requiredViewports.length) {
    errors.push(`${owner} must define exactly three Playwright projects.`);
  } else {
    for (const project of execution.projects) {
      if (!configSource.includes(`name: '${project}'`) && !configSource.includes(`name: "${project}"`)) {
        errors.push(`${owner} project ${project} is not declared in ${execution.config_file}.`);
      }
    }
  }

  if (!sameStringSet(execution?.viewports, requiredViewports)) {
    errors.push(`${owner} must declare the exact required viewport set.`);
  }
  if (execution?.retries !== 0 || !configSource.includes('retries: 0')) {
    errors.push(`${owner} must remain zero retry in both contract and Playwright config.`);
  }
  if (!nonEmptyString(execution?.workflow_marker) || !workflowSource.includes(execution.workflow_marker)) {
    errors.push(`${owner} workflow invocation marker is missing from ${execution?.workflow_file}.`);
  }
  if (nonEmptyString(execution?.npm_profile) && nonEmptyString(execution?.config_file)) {
    const command = packageScripts?.[execution.npm_profile] ?? '';
    if (!command.includes(path.basename(execution.config_file))) {
      errors.push(`${owner} npm profile does not execute ${execution.config_file}.`);
    }
  }
  if (!nonEmptyString(execution?.runtime)) errors.push(`${owner} must describe its real runtime boundary.`);
  if (!/^[0-9a-f]{40}$/u.test(execution?.validated_sha ?? '')) {
    errors.push(`${owner} must record a full validated SHA.`);
  }
  if (!Number.isInteger(execution?.validated_workflow_run_id) || execution.validated_workflow_run_id < 1) {
    errors.push(`${owner} must record a positive validated workflow run id.`);
  }

  if (!sameStringSet(mapping?.assertions, expected)) {
    errors.push(`${owner} assertions must exactly match ${JSON.stringify(expected)}.`);
  }
  if (!stringArray(mapping?.proven_assertions)) {
    errors.push(`${owner} must define concrete proven assertions.`);
  }
  if (!Array.isArray(mapping?.remaining_gaps) || mapping.remaining_gaps.length !== 0) {
    errors.push(`${owner} must have zero remaining_gaps before it is mapped.`);
  }

  return fixtureFile !== null && evidenceFile !== null;
}

export function validatePortalContentScaleEvidence({
  contract,
  manifestSurfaces,
  packageScripts,
  repoRoot = defaultRepoRoot,
}) {
  const errors = [];
  const warnings = [];

  if (contract?.schema_version !== 1) errors.push('portal content scale schema_version must be 1.');
  if (!['partial', 'complete'].includes(contract?.status)) {
    errors.push('portal content scale status must be partial or complete.');
  }
  if (!sameStringSet(contract?.classification_values, classificationValues)) {
    errors.push('portal content scale classification_values do not match the supported contract values.');
  }
  if (!Array.isArray(contract?.nonclaims) || contract.nonclaims.length < 3 || !contract.nonclaims.every(nonEmptyString)) {
    errors.push('portal content scale evidence must preserve at least three explicit nonclaims.');
  }

  const evidenceContract = contract?.evidence_contract;
  if (evidenceContract?.status !== contract?.status) {
    errors.push('evidence_contract status must match the top-level contract status.');
  }
  if (!sameStringSet(evidenceContract?.required_viewports, requiredViewports)) {
    errors.push('required_viewports must match the exact desktop, tablet and mobile contract.');
  }
  if (!sameStringSet(evidenceContract?.required_assertions, requiredAssertions)) {
    errors.push('required_assertions must match the exact content-scale assertion contract.');
  }

  const manifestById = new Map();
  for (const surface of Array.isArray(manifestSurfaces) ? manifestSurfaces : []) {
    if (!nonEmptyString(surface?.id)) {
      errors.push('Every portal manifest surface must have a stable id.');
      continue;
    }
    if (manifestById.has(surface.id)) errors.push(`Duplicate portal manifest surface id: ${surface.id}`);
    manifestById.set(surface.id, surface);
  }

  const records = contract?.surfaces && typeof contract.surfaces === 'object' ? contract.surfaces : {};
  const recordIds = Object.keys(records);
  for (const id of manifestById.keys()) {
    if (!(id in records)) errors.push(`Missing content scale classification for portal surface: ${id}`);
  }
  for (const id of recordIds) {
    if (!manifestById.has(id)) errors.push(`Content scale classification references unknown portal surface: ${id}`);
  }

  const candidateIds = [];
  for (const [id, record] of Object.entries(records)) {
    const classification = record?.classification;
    const manifest = manifestById.get(id);
    if (!classificationValues.includes(classification)) {
      errors.push(`${id} has unsupported content scale classification ${JSON.stringify(classification)}.`);
      continue;
    }
    if (!nonEmptyString(record?.rationale) || record.rationale.trim().length < 40) {
      errors.push(`${id} requires a bounded classification rationale of at least 40 characters.`);
    }

    if (manifest?.status === 'supporting_endpoint') {
      if (classification !== 'supporting_endpoint') errors.push(`${id} must remain supporting_endpoint.`);
      continue;
    }
    if (classification === 'supporting_endpoint') {
      errors.push(`${id} cannot be supporting_endpoint because the manifest declares a rendered surface.`);
      continue;
    }
    if (classification.startsWith('candidate_')) candidateIds.push(id);
  }

  const mapped = evidenceContract?.mapped_surfaces && typeof evidenceContract.mapped_surfaces === 'object'
    ? evidenceContract.mapped_surfaces
    : {};
  const gaps = evidenceContract?.gap_surfaces && typeof evidenceContract.gap_surfaces === 'object'
    ? evidenceContract.gap_surfaces
    : {};

  for (const id of Object.keys(mapped)) {
    if (!candidateIds.includes(id)) errors.push(`Orphan content scale mapping references non-candidate surface: ${id}`);
    if (id in gaps) errors.push(`${id} cannot be both mapped and an explicit gap.`);
  }
  for (const id of Object.keys(gaps)) {
    if (!candidateIds.includes(id)) errors.push(`Orphan content scale gap references non-candidate surface: ${id}`);
  }

  for (const id of candidateIds) {
    const classified = Number(id in mapped) + Number(id in gaps);
    if (classified !== 1) {
      errors.push(`${id} must be classified exactly once as mapped or gap; found ${classified}.`);
      continue;
    }

    const classification = records[id].classification;
    const expected = expectedAssertions(classification);
    if (id in mapped) {
      validateMapping(repoRoot, id, classification, mapped[id], packageScripts, errors);
      continue;
    }

    const gap = gaps[id];
    if (!nonEmptyString(gap?.reason) || gap.reason.trim().length < 80) {
      errors.push(`${id} gap requires a bounded reason of at least 80 characters.`);
    }
    if (!sameStringSet(gap?.missing, expected)) {
      errors.push(`${id} gap missing assertions must exactly match ${JSON.stringify(expected)}.`);
    }
  }

  if (contract?.status === 'complete' && Object.keys(gaps).length !== 0) {
    errors.push(`complete content scale closure requires zero gap surfaces; found ${Object.keys(gaps).length}.`);
  }
  if (contract?.status === 'partial' && Object.keys(gaps).length === 0) {
    warnings.push('partial content scale contract has no explicit gap surfaces.');
  }

  return {
    schema_version: contract?.schema_version ?? null,
    status: contract?.status ?? null,
    portal_surface_count: manifestById.size,
    classified_surface_count: recordIds.length,
    candidate_surface_count: candidateIds.length,
    mapped_surface_count: Object.keys(mapped).length,
    gap_surface_count: Object.keys(gaps).length,
    errors,
    warnings,
  };
}

export function loadRepositoryInputs(repoRoot = defaultRepoRoot) {
  const manifest = readJson(path.join(repoRoot, 'scripts/acceptance/coverage/portal-coverage-manifest.json'));
  const packageJson = readJson(path.join(repoRoot, 'scripts/acceptance/package.json'));

  return {
    contract: readJson(path.join(repoRoot, 'docs/testing/PORTAL_CONTENT_SCALE_EVIDENCE.json')),
    manifestSurfaces: Array.isArray(manifest.surfaces) ? manifest.surfaces : [],
    packageScripts: packageJson.scripts ?? {},
    repoRoot,
  };
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  let report;
  try {
    report = validatePortalContentScaleEvidence(loadRepositoryInputs());
  } catch (error) {
    report = {
      schema_version: null,
      status: null,
      portal_surface_count: 0,
      classified_surface_count: 0,
      candidate_surface_count: 0,
      mapped_surface_count: 0,
      gap_surface_count: 0,
      errors: [`Cannot validate portal content scale evidence: ${error.message}`],
      warnings: [],
    };
  }

  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.errors.length > 0) process.exitCode = 1;
}
