import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(coverageRoot, '../../..');
const defaultContractPath = path.join(coverageRoot, 'portal-evidence-dimensions.json');
const defaultManifestPath = path.join(coverageRoot, 'portal-coverage-manifest.json');
const defaultFragmentsRoot = path.join(coverageRoot, 'surfaces');

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function loadManifestSurfaces(repoRoot = defaultRepoRoot) {
  const manifest = readJson(path.join(repoRoot, path.relative(defaultRepoRoot, defaultManifestPath)));
  const fragmentsRoot = path.join(repoRoot, path.relative(defaultRepoRoot, defaultFragmentsRoot));
  const fragments = [];

  if (fs.existsSync(fragmentsRoot)) {
    for (const entry of fs.readdirSync(fragmentsRoot, { withFileTypes: true })
      .filter((candidate) => candidate.isFile() && candidate.name.endsWith('.json'))
      .sort((left, right) => left.name.localeCompare(right.name))) {
      const value = readJson(path.join(fragmentsRoot, entry.name));
      const surfaces = Array.isArray(value) ? value : value.surfaces;
      if (Array.isArray(surfaces)) fragments.push(...surfaces);
    }
  }

  return [...(Array.isArray(manifest.surfaces) ? manifest.surfaces : []), ...fragments];
}

function validateEvidence(repoRoot, owner, evidence, errors) {
  if (!evidence || typeof evidence.file !== 'string' || evidence.file.trim() === '') {
    errors.push(`${owner} evidence must define a file.`);
    return;
  }

  const absolute = path.resolve(repoRoot, evidence.file);
  if (!absolute.startsWith(`${repoRoot}${path.sep}`)) {
    errors.push(`${owner} evidence escapes repository root: ${evidence.file}`);
    return;
  }
  if (!fs.existsSync(absolute) || !fs.statSync(absolute).isFile()) {
    errors.push(`${owner} references missing evidence file ${evidence.file}.`);
    return;
  }

  if (!Array.isArray(evidence.markers) || evidence.markers.length === 0) {
    errors.push(`${owner} evidence ${evidence.file} must define non-empty markers.`);
    return;
  }

  const content = fs.readFileSync(absolute, 'utf8');
  for (const marker of evidence.markers) {
    if (typeof marker !== 'string' || marker.trim() === '') {
      errors.push(`${owner} evidence ${evidence.file} contains an empty marker.`);
    } else if (!content.includes(marker)) {
      errors.push(`${owner} evidence marker not found in ${evidence.file}: ${marker}`);
    }
  }
}

function validateMarkerSource(repoRoot, owner, source, errors) {
  if (!source || typeof source.file !== 'string' || !Array.isArray(source.markers) || source.markers.length === 0) {
    errors.push(`${owner} must define a file and non-empty markers.`);
    return;
  }
  validateEvidence(repoRoot, owner, source, errors);
}

function projectIndex(profiles, errors, repoRoot) {
  const profilesById = new Map();
  const projectsByProfile = new Map();

  for (const profile of profiles) {
    if (typeof profile?.id !== 'string' || profile.id.trim() === '') {
      errors.push('Every execution profile must have a stable id.');
      continue;
    }
    if (profilesById.has(profile.id)) {
      errors.push(`Duplicate execution profile id: ${profile.id}`);
      continue;
    }
    profilesById.set(profile.id, profile);

    if (typeof profile.config_file !== 'string' || profile.config_file.trim() === '') {
      errors.push(`${profile.id} must define config_file.`);
      continue;
    }
    const configAbsolute = path.resolve(repoRoot, profile.config_file);
    if (!configAbsolute.startsWith(`${repoRoot}${path.sep}`)
      || !fs.existsSync(configAbsolute)
      || !fs.statSync(configAbsolute).isFile()) {
      errors.push(`${profile.id} references missing config ${profile.config_file}.`);
      continue;
    }

    const configSource = fs.readFileSync(configAbsolute, 'utf8');
    const projects = new Map();
    if (!Array.isArray(profile.projects) || profile.projects.length === 0) {
      errors.push(`${profile.id} must define at least one Playwright project.`);
    } else {
      for (const project of profile.projects) {
        if (typeof project?.name !== 'string' || project.name.trim() === '') {
          errors.push(`${profile.id} contains a project without a name.`);
          continue;
        }
        if (projects.has(project.name)) {
          errors.push(`${profile.id} has duplicate project ${project.name}.`);
          continue;
        }
        projects.set(project.name, project);
        const singleQuoted = `name: '${project.name}'`;
        const doubleQuoted = `name: "${project.name}"`;
        if (!configSource.includes(singleQuoted) && !configSource.includes(doubleQuoted)) {
          errors.push(`${profile.id} project ${project.name} is not present in ${profile.config_file}.`);
        }
        if (!['chromium', 'firefox', 'webkit'].includes(project.browser)) {
          errors.push(`${profile.id} project ${project.name} has unsupported browser ${JSON.stringify(project.browser)}.`);
        }
        if (typeof project.viewport !== 'string' || project.viewport.trim() === '') {
          errors.push(`${profile.id} project ${project.name} must define a viewport.`);
        }
      }
    }
    projectsByProfile.set(profile.id, projects);

    if (profile.blocking === true) {
      validateMarkerSource(repoRoot, `${profile.id} zero-retry proof`, profile.zero_retry, errors);
      validateMarkerSource(repoRoot, `${profile.id} blocking invocation`, profile.invocation, errors);
    } else if (profile.blocking !== false) {
      errors.push(`${profile.id} must define blocking as a boolean.`);
    }
  }

  return { profilesById, projectsByProfile };
}

function validateMappingEvidence(repoRoot, owner, mapping, evidenceById, errors) {
  if (!mapping || typeof mapping !== 'object') {
    errors.push(`${owner} mapping must be an object.`);
    return;
  }
  const evidence = evidenceById.get(mapping.evidence);
  if (!evidence) {
    errors.push(`${owner} references unknown evidence ${JSON.stringify(mapping.evidence)}.`);
    return;
  }
  validateEvidence(repoRoot, owner, evidence, errors);
}

function getProject(owner, mapping, profilesById, projectsByProfile, errors) {
  const profile = profilesById.get(mapping.profile);
  if (!profile) {
    errors.push(`${owner} references unknown profile ${JSON.stringify(mapping.profile)}.`);
    return { profile: null, project: null };
  }
  const project = projectsByProfile.get(mapping.profile)?.get(mapping.project);
  if (!project) {
    errors.push(`${owner} references unknown project ${JSON.stringify(mapping.project)} in ${mapping.profile}.`);
  }
  return { profile, project };
}

export function validatePortalEvidenceDimensions({ contract, manifestSurfaces, repoRoot = defaultRepoRoot }) {
  const errors = [];
  const warnings = [];

  if (!contract || contract.schema_version !== 1) errors.push('portal evidence dimension schema_version must be 1.');

  const allowedViewports = new Set(Array.isArray(contract?.allowed_viewports) ? contract.allowed_viewports : []);
  const browserContracts = contract?.browser_contracts && typeof contract.browser_contracts === 'object'
    ? contract.browser_contracts
    : {};
  if (allowedViewports.size === 0) errors.push('allowed_viewports must be non-empty.');
  if (Object.keys(browserContracts).length === 0) errors.push('browser_contracts must be non-empty.');

  const profiles = Array.isArray(contract?.profiles) ? contract.profiles : [];
  const { profilesById, projectsByProfile } = projectIndex(profiles, errors, repoRoot);

  const manifestById = new Map();
  for (const surface of manifestSurfaces) {
    if (manifestById.has(surface.id)) errors.push(`Duplicate manifest surface id: ${surface.id}`);
    manifestById.set(surface.id, surface);
  }

  const records = Array.isArray(contract?.surfaces) ? contract.surfaces : [];
  const recordsById = new Map();
  for (const record of records) {
    if (typeof record?.id !== 'string' || record.id.trim() === '') {
      errors.push('Every dimension record must have a stable surface_id.');
      continue;
    }
    if (recordsById.has(record.id)) {
      errors.push(`Duplicate dimension record: ${record.id}`);
      continue;
    }
    recordsById.set(record.id, record);
    if (!manifestById.has(record.id)) errors.push(`Dimension record references unknown surface: ${record.id}`);
  }

  for (const surfaceId of manifestById.keys()) {
    if (!recordsById.has(surfaceId)) errors.push(`Missing dimension record for surface: ${surfaceId}`);
  }

  for (const [surfaceId, record] of recordsById) {
    const manifest = manifestById.get(surfaceId);
    if (!manifest) continue;

    if (manifest.status === 'supporting_endpoint') {
      if (record.kind !== 'supporting_endpoint') errors.push(`${surfaceId} must be classified as supporting_endpoint in the dimension contract.`);
      if (typeof record.rationale !== 'string' || record.rationale.trim().length < 40) {
        errors.push(`${surfaceId} supporting endpoint requires a bounded rationale.`);
      }
      continue;
    }

    if (manifest.status !== 'covered') errors.push(`${surfaceId} is ${manifest.status}; only covered rendered surfaces may use a rendered dimension record.`);
    if (record.kind !== 'rendered') {
      errors.push(`${surfaceId} must be classified as rendered.`);
      continue;
    }
    if (typeof record.critical !== 'boolean') errors.push(`${surfaceId} must define critical as a boolean.`);

    const evidenceById = new Map();
    for (const evidence of Array.isArray(record.evidence) ? record.evidence : []) {
      if (typeof evidence?.id !== 'string' || evidence.id.trim() === '') {
        errors.push(`${surfaceId} contains evidence without a stable id.`);
        continue;
      }
      if (evidenceById.has(evidence.id)) {
        errors.push(`${surfaceId} has duplicate evidence id ${evidence.id}.`);
        continue;
      }
      evidenceById.set(evidence.id, evidence);
      validateEvidence(repoRoot, `${surfaceId} evidence ${evidence.id}`, evidence, errors);
    }

    const declaredViewports = Array.isArray(manifest.viewports) ? manifest.viewports : [];
    for (const viewport of declaredViewports) {
      if (!allowedViewports.has(viewport)) errors.push(`${surfaceId} declares unknown viewport ${viewport}.`);
    }
    const mappedViewports = record.viewports && typeof record.viewports === 'object' ? Object.keys(record.viewports) : [];
    if (JSON.stringify([...declaredViewports].sort()) !== JSON.stringify([...mappedViewports].sort())) {
      errors.push(`${surfaceId} viewport mappings do not match declared viewports.`);
    }

    for (const [viewport, mapping] of Object.entries(record.viewports ?? {})) {
      const owner = `${surfaceId} viewport ${viewport}`;
      if (!allowedViewports.has(viewport)) errors.push(`${owner} is not canonical.`);
      validateMappingEvidence(repoRoot, owner, mapping, evidenceById, errors);
      const { profile, project } = getProject(owner, mapping, profilesById, projectsByProfile, errors);
      if (!profile || !project) continue;
      if (record.critical === true && profile.blocking !== true) {
        errors.push(`${owner} must use a blocking profile because the surface is critical.`);
      }
      if (mapping.mode === 'project') {
        if (project.viewport !== viewport) errors.push(`${owner} project ${project.name} executes ${project.viewport}, not ${viewport}.`);
      } else if (mapping.mode === 'test-controlled') {
        const evidence = evidenceById.get(mapping.evidence);
        if (!Array.isArray(evidence?.markers) || evidence.markers.length < 2) {
          errors.push(`${owner} test-controlled mapping must include a stable scenario marker and an exact viewport marker.`);
        }
      } else {
        errors.push(`${owner} has unsupported mode ${JSON.stringify(mapping.mode)}.`);
      }
    }

    const declaredBrowsers = Array.isArray(manifest.browsers) ? manifest.browsers : [];
    for (const browser of declaredBrowsers) {
      if (!Object.prototype.hasOwnProperty.call(browserContracts, browser)) errors.push(`${surfaceId} declares unknown browser/profile id ${browser}.`);
    }
    const mappedBrowsers = record.browsers && typeof record.browsers === 'object' ? Object.keys(record.browsers) : [];
    if (JSON.stringify([...declaredBrowsers].sort()) !== JSON.stringify([...mappedBrowsers].sort())) {
      errors.push(`${surfaceId} browser mappings do not match declared browsers.`);
    }

    for (const [browserId, mapping] of Object.entries(record.browsers ?? {})) {
      const owner = `${surfaceId} browser ${browserId}`;
      validateMappingEvidence(repoRoot, owner, mapping, evidenceById, errors);
      const profile = profilesById.get(mapping.profile);
      if (!profile) {
        errors.push(`${owner} references unknown profile ${JSON.stringify(mapping.profile)}.`);
        continue;
      }
      if (!Array.isArray(mapping.projects) || mapping.projects.length === 0) {
        errors.push(`${owner} must reference at least one project.`);
        continue;
      }
      const actualEngines = new Set();
      for (const projectName of mapping.projects) {
        const project = projectsByProfile.get(mapping.profile)?.get(projectName);
        if (!project) errors.push(`${owner} references unknown project ${JSON.stringify(projectName)} in ${mapping.profile}.`);
        else actualEngines.add(project.browser);
      }
      for (const expected of new Set(browserContracts[browserId] ?? [])) {
        if (!actualEngines.has(expected)) errors.push(`${owner} does not prove required ${expected} execution.`);
      }
    }

    const portability = record.portability;
    if (!portability || !['covered', 'excluded'].includes(portability.status)) {
      errors.push(`${surfaceId} must define portability as covered or excluded.`);
    } else if (portability.status === 'excluded') {
      if (typeof portability.rationale !== 'string' || portability.rationale.trim().length < 80) {
        errors.push(`${surfaceId} portability exclusion requires a bounded risk-based rationale.`);
      }
      const declaresCrossEngine = declaredBrowsers.some((browserId) =>
        (browserContracts[browserId] ?? []).some((engine) => engine === 'firefox' || engine === 'webkit'));
      if (declaresCrossEngine) errors.push(`${surfaceId} declares cross-engine browser coverage but portability is excluded.`);
    } else {
      validateMappingEvidence(repoRoot, `${surfaceId} portability`, portability, evidenceById, errors);
      const engines = new Set();
      if (!Array.isArray(portability.projects) || portability.projects.length === 0) {
        errors.push(`${surfaceId} covered portability must reference projects.`);
      } else {
        for (const projectName of portability.projects) {
          const project = projectsByProfile.get(portability.profile)?.get(projectName);
          if (!project) errors.push(`${surfaceId} portability references unknown project ${JSON.stringify(projectName)}.`);
          else engines.add(project.browser);
        }
      }
      if (!engines.has('firefox') || !engines.has('webkit')) errors.push(`${surfaceId} covered portability must prove both Firefox and WebKit.`);
    }
  }

  return {
    schema_version: contract?.schema_version ?? null,
    manifest_surface_count: manifestById.size,
    dimension_record_count: recordsById.size,
    execution_profile_count: profilesById.size,
    critical_surface_count: records.filter((record) => record.kind === 'rendered' && record.critical === true).length,
    errors,
    warnings,
  };
}

export function loadBaseline(repoRoot = defaultRepoRoot) {
  const contractPath = path.join(repoRoot, path.relative(defaultRepoRoot, defaultContractPath));
  const contract = readJson(contractPath);
  const surfaces = [];
  for (const fragment of Array.isArray(contract.surface_fragments) ? contract.surface_fragments : []) {
    const fragmentPath = path.resolve(path.dirname(contractPath), fragment);
    if (!fragmentPath.startsWith(`${repoRoot}${path.sep}`)) throw new Error(`Dimension fragment escapes repository root: ${fragment}`);
    const value = readJson(fragmentPath);
    if (!Array.isArray(value.surfaces)) throw new Error(`Dimension fragment ${fragment} must contain a surfaces array.`);
    surfaces.push(...value.surfaces);
  }
  contract.surfaces = surfaces;
  return { contract, manifestSurfaces: loadManifestSurfaces(repoRoot), repoRoot };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  let report;
  try {
    report = validatePortalEvidenceDimensions(loadBaseline());
  } catch (error) {
    report = {schema_version: null, manifest_surface_count: 0, dimension_record_count: 0, execution_profile_count: 0, critical_surface_count: 0, errors: [error.message], warnings: []};
  }
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.errors.length > 0) process.exitCode = 1;
}
