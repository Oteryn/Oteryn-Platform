import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(coverageRoot, '../../..');
const manifestPath = path.join(coverageRoot, 'portal-coverage-manifest.json');
const fragmentRoot = path.join(coverageRoot, 'surfaces');
const contractPath = path.join(repoRoot, 'docs/testing/PORTAL_EVIDENCE_DIMENSIONS.json');
const packagePath = path.join(repoRoot, 'scripts/acceptance/package.json');
const playwrightPath = path.join(repoRoot, 'scripts/acceptance/playwright.config.mjs');

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function readFragments() {
  if (!fs.existsSync(fragmentRoot)) return [];

  return fs.readdirSync(fragmentRoot, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .sort((left, right) => left.name.localeCompare(right.name))
    .flatMap((entry) => {
      const value = readJson(path.join(fragmentRoot, entry.name));
      return Array.isArray(value) ? value : value.surfaces ?? [];
    });
}

function projectNames(playwrightConfig) {
  return new Set([...playwrightConfig.matchAll(/\bname:\s*['"]([^'"]+)['"]/g)].map((match) => match[1]));
}

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() !== '';
}

function checkDimensionRule({ kind, id, rule, projects, npmScripts, errors }) {
  if (!rule || typeof rule !== 'object' || Array.isArray(rule)) {
    errors.push(`${kind} ${id} must define an object rule.`);
    return;
  }

  const referencedProjects = Array.isArray(rule.playwright_projects) ? rule.playwright_projects : [];
  const referencedProfiles = Array.isArray(rule.npm_profiles) ? rule.npm_profiles : [];

  if (referencedProjects.length === 0 || referencedProjects.some((value) => !nonEmptyString(value))) {
    errors.push(`${kind} ${id} must reference at least one Playwright project.`);
  }
  if (referencedProfiles.length === 0 || referencedProfiles.some((value) => !nonEmptyString(value))) {
    errors.push(`${kind} ${id} must reference at least one npm profile.`);
  }

  for (const project of referencedProjects) {
    if (!projects.has(project)) errors.push(`${kind} ${id} references missing Playwright project ${project}.`);
  }
  for (const profile of referencedProfiles) {
    if (!Object.hasOwn(npmScripts, profile)) errors.push(`${kind} ${id} references missing npm profile ${profile}.`);
  }
}

export function validatePortalEvidenceDimensions({
  manifest,
  fragments = [],
  contract,
  packageJson,
  playwrightConfig,
  evidenceContent,
}) {
  const errors = [];
  const warnings = [];
  const surfaces = [...(manifest.surfaces ?? []), ...fragments];
  const projects = projectNames(playwrightConfig);
  const npmScripts = packageJson.scripts ?? {};
  const viewportRules = contract.viewport_dimensions ?? {};
  const browserRules = contract.browser_dimensions ?? {};
  const policy = contract.surface_evidence_policy ?? {};
  const surfaceIds = new Set();
  let dimensionMappingCount = 0;

  if (contract.schema_version !== 1) errors.push('PORTAL_EVIDENCE_DIMENSIONS schema_version must be 1.');
  if (!Array.isArray(surfaces) || surfaces.length === 0) errors.push('Portal evidence validation requires at least one surface.');

  for (const [id, rule] of Object.entries(viewportRules)) {
    checkDimensionRule({ kind: 'Viewport', id, rule, projects, npmScripts, errors });
    if (!Number.isInteger(rule.width) || rule.width <= 0 || !Number.isInteger(rule.height) || rule.height <= 0) {
      errors.push(`Viewport ${id} must define positive integer width and height.`);
    }
  }
  for (const [id, rule] of Object.entries(browserRules)) {
    checkDimensionRule({ kind: 'Browser', id, rule, projects, npmScripts, errors });
    if (!Array.isArray(rule.engines) || rule.engines.length === 0 || rule.engines.some((value) => !nonEmptyString(value))) {
      errors.push(`Browser ${id} must define at least one engine.`);
    }
  }

  const coveredStatus = policy.covered_status ?? 'covered';
  const supportingStatuses = new Set(policy.supporting_statuses ?? []);
  const requiredLayer = policy.required_evidence_layer ?? 'playwright';
  const requiredPrefix = policy.required_file_prefix ?? 'scripts/acceptance/tests/';
  const minimumMarkers = Number.isInteger(policy.minimum_stable_markers) ? policy.minimum_stable_markers : 1;
  const rationaleField = policy.non_rendered_rationale_field ?? 'dimension_exclusion_rationale';

  for (const surface of surfaces) {
    if (!nonEmptyString(surface.id)) {
      errors.push('Every portal surface must define a stable id for dimension validation.');
      continue;
    }
    if (surfaceIds.has(surface.id)) errors.push(`Duplicate portal surface id in dimension validation: ${surface.id}`);
    surfaceIds.add(surface.id);

    const viewports = Array.isArray(surface.viewports) ? surface.viewports : [];
    const browsers = Array.isArray(surface.browsers) ? surface.browsers : [];

    for (const viewport of viewports) {
      if (!Object.hasOwn(viewportRules, viewport)) {
        errors.push(`${surface.id} declares unknown viewport dimension ${JSON.stringify(viewport)}.`);
      } else {
        dimensionMappingCount += 1;
      }
    }
    for (const browser of browsers) {
      if (!Object.hasOwn(browserRules, browser)) {
        errors.push(`${surface.id} declares unknown browser dimension ${JSON.stringify(browser)}.`);
      } else {
        dimensionMappingCount += 1;
      }
    }

    if (surface.status === coveredStatus) {
      if (viewports.length === 0) errors.push(`${surface.id} is covered but declares no viewport dimensions.`);
      if (browsers.length === 0) errors.push(`${surface.id} is covered but declares no browser dimensions.`);
      if (!Array.isArray(surface.evidence_layers) || !surface.evidence_layers.includes(requiredLayer)) {
        errors.push(`${surface.id} is covered but does not declare the ${requiredLayer} evidence layer.`);
      }

      let stableBrowserMarkerCount = 0;
      for (const evidence of Array.isArray(surface.evidence) ? surface.evidence : []) {
        if (!nonEmptyString(evidence?.file) || !evidence.file.startsWith(requiredPrefix)) continue;
        const content = evidenceContent(evidence.file);
        if (content === null) {
          errors.push(`${surface.id} references missing browser evidence file ${evidence.file}.`);
          continue;
        }
        for (const marker of Array.isArray(evidence.markers) ? evidence.markers : []) {
          if (!nonEmptyString(marker)) continue;
          if (!content.includes(marker)) {
            errors.push(`${surface.id} browser evidence marker not found in ${evidence.file}: ${marker}`);
          } else {
            stableBrowserMarkerCount += 1;
          }
        }
      }
      if (stableBrowserMarkerCount < minimumMarkers) {
        errors.push(`${surface.id} is covered but has fewer than ${minimumMarkers} stable browser evidence markers.`);
      }
    } else if (supportingStatuses.has(surface.status)) {
      if ((viewports.length > 0 || browsers.length > 0) && !nonEmptyString(surface[rationaleField])) {
        errors.push(`${surface.id} is ${surface.status} with declared dimensions but lacks ${rationaleField}.`);
      }
    }
  }

  const criticalViewports = new Set(contract.critical_required_viewports ?? []);
  for (const criticalId of contract.critical_rendered_surfaces ?? []) {
    const surface = surfaces.find((candidate) => candidate.id === criticalId);
    if (!surface) {
      errors.push(`Critical rendered surface does not exist: ${criticalId}`);
      continue;
    }
    if (surface.status !== coveredStatus) errors.push(`Critical rendered surface ${criticalId} must be covered.`);
    for (const viewport of criticalViewports) {
      if (!surface.viewports?.includes(viewport)) errors.push(`Critical rendered surface ${criticalId} lacks required viewport ${viewport}.`);
    }
    if (!surface.browsers?.includes('chromium-primary')) {
      errors.push(`Critical rendered surface ${criticalId} lacks chromium-primary browser evidence.`);
    }
  }

  return {
    schema_version: contract.schema_version,
    surface_count: surfaces.length,
    viewport_dimension_count: Object.keys(viewportRules).length,
    browser_dimension_count: Object.keys(browserRules).length,
    playwright_project_count: projects.size,
    npm_profile_count: Object.keys(npmScripts).length,
    dimension_mapping_count: dimensionMappingCount,
    errors,
    warnings,
  };
}

function loadBaseline() {
  const manifest = readJson(manifestPath);
  const contract = readJson(contractPath);
  const packageJson = readJson(packagePath);
  const playwrightConfig = fs.readFileSync(playwrightPath, 'utf8');

  return {
    manifest,
    fragments: readFragments(),
    contract,
    packageJson,
    playwrightConfig,
    evidenceContent(file) {
      const absolute = path.resolve(repoRoot, file);
      if (!absolute.startsWith(`${repoRoot}${path.sep}`) || !fs.existsSync(absolute) || !fs.statSync(absolute).isFile()) return null;
      return fs.readFileSync(absolute, 'utf8');
    },
  };
}

export { loadBaseline };

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const report = validatePortalEvidenceDimensions(loadBaseline());
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.errors.length > 0) process.exitCode = 1;
}
