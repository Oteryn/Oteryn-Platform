import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(coverageRoot, '../../..');
const requiredStates = ['normal', 'missing', 'broken_or_integrity_failed', 'no_image'];

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() !== '';
}

function stringArray(value) {
  return Array.isArray(value) && value.length > 0 && value.every(nonEmptyString);
}

function readFragments(root) {
  if (!fs.existsSync(root)) return [];

  return fs.readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .sort((left, right) => left.name.localeCompare(right.name))
    .flatMap((entry) => {
      const value = readJson(path.join(root, entry.name));
      return Array.isArray(value) ? value : (Array.isArray(value.surfaces) ? value.surfaces : []);
    });
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

function validateMarkers(repoRoot, owner, source, errors) {
  const absolute = safeFile(repoRoot, owner, source?.file, errors);
  if (absolute === null) return;

  if (!stringArray(source?.markers)) {
    errors.push(`${owner} must define non-empty evidence markers.`);
    return;
  }
  if (new Set(source.markers).size !== source.markers.length) {
    errors.push(`${owner} contains duplicate evidence markers.`);
  }

  const content = fs.readFileSync(absolute, 'utf8');
  for (const marker of source.markers) {
    if (!content.includes(marker)) {
      errors.push(`${owner} evidence marker not found in ${source.file}: ${marker}`);
    }
  }
}

function loadDimensionInputs(repoRoot) {
  const dimensions = readJson(path.join(repoRoot, 'scripts/acceptance/coverage/portal-evidence-dimensions.json'));
  const surfaceFragments = Array.isArray(dimensions.surface_fragments)
    ? dimensions.surface_fragments.flatMap((relativeFile) => {
      const value = readJson(path.join(repoRoot, 'scripts/acceptance/coverage', relativeFile));
      return Array.isArray(value) ? value : (Array.isArray(value.surfaces) ? value.surfaces : []);
    })
    : [];

  return {
    dimensions,
    dimensionSurfaces: surfaceFragments,
  };
}

function validateExecution(repoRoot, owner, evidence, profilesById, errors) {
  if (!nonEmptyString(evidence?.profile)) {
    errors.push(`${owner} must define an execution profile.`);
    return;
  }
  if (!stringArray(evidence?.projects)) {
    errors.push(`${owner} must define at least one Playwright project.`);
    return;
  }
  if (new Set(evidence.projects).size !== evidence.projects.length) {
    errors.push(`${owner} contains duplicate Playwright projects.`);
  }

  const profile = profilesById.get(evidence.profile);
  if (!profile) {
    errors.push(`${owner} references unknown execution profile ${JSON.stringify(evidence.profile)}.`);
    return;
  }
  if (profile.blocking !== true) {
    errors.push(`${owner} must use a blocking execution profile; ${profile.id} is not blocking.`);
  }

  validateMarkers(repoRoot, `${owner} zero-retry proof`, profile.zero_retry, errors);
  validateMarkers(repoRoot, `${owner} workflow invocation`, profile.invocation, errors);

  const configAbsolute = safeFile(repoRoot, `${owner} config`, profile.config_file, errors);
  const declaredProjects = new Map(
    (Array.isArray(profile.projects) ? profile.projects : [])
      .filter((project) => nonEmptyString(project?.name))
      .map((project) => [project.name, project]),
  );
  const configSource = configAbsolute === null ? '' : fs.readFileSync(configAbsolute, 'utf8');

  for (const projectName of evidence.projects) {
    const project = declaredProjects.get(projectName);
    if (!project) {
      errors.push(`${owner} references unknown Playwright project ${JSON.stringify(projectName)} in ${profile.id}.`);
      continue;
    }

    const singleQuoted = `name: '${projectName}'`;
    const doubleQuoted = `name: "${projectName}"`;
    if (!configSource.includes(singleQuoted) && !configSource.includes(doubleQuoted)) {
      errors.push(`${owner} project ${projectName} is not declared in ${profile.config_file}.`);
    }
  }
}

export function validatePortalMediaStateEvidence({
  contract,
  manifestSurfaces,
  dimensionContract,
  dimensionSurfaces,
  repoRoot = defaultRepoRoot,
}) {
  const errors = [];
  const warnings = [];

  if (contract?.schema_version !== 1) errors.push('portal media evidence schema_version must be 1.');
  if (contract?.issue !== 357) errors.push('portal media evidence issue must be 357.');
  if (contract?.parent_issue !== 326) errors.push('portal media evidence parent_issue must be 326.');
  if (JSON.stringify(contract?.required_rendered_states) !== JSON.stringify(requiredStates)) {
    errors.push(`required_rendered_states must be exactly ${JSON.stringify(requiredStates)}.`);
  }
  if (!Array.isArray(contract?.nonclaims) || contract.nonclaims.length < 3 || !contract.nonclaims.every(nonEmptyString)) {
    errors.push('portal media evidence must preserve at least three explicit nonclaims.');
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

  const dimensionsById = new Map();
  for (const surface of Array.isArray(dimensionSurfaces) ? dimensionSurfaces : []) {
    if (!nonEmptyString(surface?.id)) continue;
    if (dimensionsById.has(surface.id)) errors.push(`Duplicate dimension surface id: ${surface.id}`);
    dimensionsById.set(surface.id, surface);
  }

  for (const id of manifestById.keys()) {
    if (!dimensionsById.has(id)) errors.push(`Portal surface has no dimension contract: ${id}`);
  }
  for (const id of dimensionsById.keys()) {
    if (!manifestById.has(id)) errors.push(`Dimension contract references unknown portal surface: ${id}`);
  }

  const profiles = Array.isArray(dimensionContract?.profiles) ? dimensionContract.profiles : [];
  const profilesById = new Map();
  for (const profile of profiles) {
    if (!nonEmptyString(profile?.id)) {
      errors.push('Every execution profile must have a stable id.');
      continue;
    }
    if (profilesById.has(profile.id)) errors.push(`Duplicate execution profile id: ${profile.id}`);
    profilesById.set(profile.id, profile);
  }

  const records = Array.isArray(contract?.surfaces) ? contract.surfaces : [];
  const recordsById = new Map();
  let mediaConsumerCount = 0;
  let supportingEndpointCount = 0;
  let evidencedStateCount = 0;
  let gapStateCount = 0;

  for (const record of records) {
    if (!nonEmptyString(record?.id)) {
      errors.push('Every media classification record must have a stable id.');
      continue;
    }
    if (recordsById.has(record.id)) {
      errors.push(`Duplicate media classification record: ${record.id}`);
      continue;
    }
    recordsById.set(record.id, record);
    if (!manifestById.has(record.id)) errors.push(`Media classification references unknown portal surface: ${record.id}`);
  }

  for (const id of manifestById.keys()) {
    if (!recordsById.has(id)) errors.push(`Missing media classification for portal surface: ${id}`);
  }

  for (const [id, record] of recordsById) {
    const manifest = manifestById.get(id);
    if (!manifest) continue;

    const classification = record.classification;
    if (manifest.status === 'supporting_endpoint') {
      if (classification !== 'supporting_endpoint') {
        errors.push(`${id} must be classified as supporting_endpoint.`);
        continue;
      }
      supportingEndpointCount += 1;
      if (!nonEmptyString(record.rationale) || record.rationale.trim().length < 80) {
        errors.push(`${id} supporting_endpoint requires a bounded rationale of at least 80 characters.`);
      }
      if (record.states !== undefined) errors.push(`${id} supporting_endpoint must not define rendered states.`);
      continue;
    }

    if (manifest.status !== 'covered') {
      errors.push(`${id} has unsupported portal status ${JSON.stringify(manifest.status)} for this delivered-surface contract.`);
      continue;
    }

    if (!['media_consumer', 'not_applicable'].includes(classification)) {
      errors.push(`${id} must be classified as media_consumer or not_applicable.`);
      continue;
    }

    if (classification === 'not_applicable') {
      if (!nonEmptyString(record.rationale) || record.rationale.trim().length < 60) {
        errors.push(`${id} not_applicable classification requires a bounded rationale of at least 60 characters.`);
      }
      if (record.states !== undefined) errors.push(`${id} not_applicable classification must not define rendered states.`);
      continue;
    }

    mediaConsumerCount += 1;
    const states = record.states && typeof record.states === 'object' ? record.states : {};
    const actualStateNames = Object.keys(states).sort();
    const expectedStateNames = [...requiredStates].sort();
    if (JSON.stringify(actualStateNames) !== JSON.stringify(expectedStateNames)) {
      errors.push(`${id} rendered state set does not match required_rendered_states.`);
    }

    for (const stateName of requiredStates) {
      const owner = `${id}.${stateName}`;
      const state = states[stateName];
      if (!state || !['evidenced', 'gap'].includes(state.status)) {
        errors.push(`${owner} must be explicitly evidenced or gap.`);
        continue;
      }

      if (state.status === 'gap') {
        gapStateCount += 1;
        if (!nonEmptyString(state.reason) || state.reason.trim().length < 80) {
          errors.push(`${owner} gap requires a bounded reason of at least 80 characters.`);
        }
        if (state.evidence !== undefined) errors.push(`${owner} gap must not contain executable evidence.`);
        continue;
      }

      evidencedStateCount += 1;
      if (state.reason !== undefined) errors.push(`${owner} evidenced state must not define a gap reason.`);
      validateMarkers(repoRoot, owner, state.evidence, errors);
      validateExecution(repoRoot, owner, state.evidence, profilesById, errors);
    }
  }

  if (mediaConsumerCount === 0) errors.push('At least one rendered media_consumer must be declared.');
  if (supportingEndpointCount === 0) errors.push('At least one supporting_endpoint must be declared.');

  return {
    schema_version: contract?.schema_version ?? null,
    portal_surface_count: manifestById.size,
    classified_surface_count: recordsById.size,
    execution_profile_count: profilesById.size,
    media_consumer_count: mediaConsumerCount,
    supporting_endpoint_count: supportingEndpointCount,
    evidenced_state_count: evidencedStateCount,
    gap_state_count: gapStateCount,
    errors,
    warnings,
  };
}

export function loadRepositoryInputs(repoRoot = defaultRepoRoot) {
  const manifest = readJson(path.join(repoRoot, 'scripts/acceptance/coverage/portal-coverage-manifest.json'));
  const manifestFragments = readFragments(path.join(repoRoot, 'scripts/acceptance/coverage/surfaces'));
  const { dimensions, dimensionSurfaces } = loadDimensionInputs(repoRoot);

  return {
    contract: readJson(path.join(repoRoot, 'docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json')),
    manifestSurfaces: [...(Array.isArray(manifest.surfaces) ? manifest.surfaces : []), ...manifestFragments],
    dimensionContract: dimensions,
    dimensionSurfaces,
    repoRoot,
  };
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  let report;
  try {
    report = validatePortalMediaStateEvidence(loadRepositoryInputs());
  } catch (error) {
    report = {
      schema_version: null,
      portal_surface_count: 0,
      classified_surface_count: 0,
      execution_profile_count: 0,
      media_consumer_count: 0,
      supporting_endpoint_count: 0,
      evidenced_state_count: 0,
      gap_state_count: 0,
      errors: [`Cannot validate portal media state evidence: ${error.message}`],
      warnings: [],
    };
  }

  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.errors.length > 0) process.exitCode = 1;
}
