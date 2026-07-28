import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(coverageRoot, '../../..');
const manifestPath = path.join(coverageRoot, 'portal-coverage-manifest.json');
const surfaceFragmentRoot = path.join(coverageRoot, 'surfaces');
const strict = process.argv.includes('--strict');
const manifestOnly = process.argv.includes('--manifest-only');

const errors = [];
const warnings = [];

function readJson(file) {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (error) {
    errors.push(`Cannot parse ${path.relative(repoRoot, file)}: ${error.message}`);
    return null;
  }
}

function readSurfaceFragments() {
  if (!fs.existsSync(surfaceFragmentRoot)) return [];

  const fragments = [];
  for (const entry of fs.readdirSync(surfaceFragmentRoot, { withFileTypes: true })
    .filter((candidate) => candidate.isFile() && candidate.name.endsWith('.json'))
    .sort((left, right) => left.name.localeCompare(right.name))) {
    const file = path.join(surfaceFragmentRoot, entry.name);
    const value = readJson(file);
    if (value === null) continue;

    const surfaces = Array.isArray(value) ? value : value.surfaces;
    if (!Array.isArray(surfaces) || surfaces.length === 0) {
      errors.push(`${path.relative(repoRoot, file)} must contain a non-empty surface array.`);
      continue;
    }

    fragments.push(...surfaces);
  }

  return fragments;
}

function requireStringArray(surface, field) {
  const value = surface[field];
  if (!Array.isArray(value) || value.length === 0 || value.some((item) => typeof item !== 'string' || item.trim() === '')) {
    errors.push(`${surface.id ?? '[unknown]'} must define a non-empty string array for ${field}.`);
    return [];
  }
  return value;
}

function exclusionMatches(name, exclusion) {
  if (typeof exclusion?.exact === 'string') return name === exclusion.exact;
  if (typeof exclusion?.prefix === 'string') return name.startsWith(exclusion.prefix);
  return false;
}

const manifest = readJson(manifestPath);
if (!manifest) {
  process.exitCode = 1;
} else {
  manifest.surfaces = [...(Array.isArray(manifest.surfaces) ? manifest.surfaces : []), ...readSurfaceFragments()];

  if (manifest.schema_version !== 1) errors.push('portal coverage schema_version must be 1.');
  if (!Array.isArray(manifest.allowed_statuses) || manifest.allowed_statuses.length === 0) {
    errors.push('allowed_statuses must be a non-empty array.');
  }
  if (!Array.isArray(manifest.surfaces) || manifest.surfaces.length === 0) {
    errors.push('surfaces must be a non-empty array.');
  }

  const allowedStatuses = new Set(manifest.allowed_statuses ?? []);
  const surfaceIds = new Set();
  const routeOwners = new Map();

  for (const surface of manifest.surfaces ?? []) {
    if (typeof surface.id !== 'string' || surface.id.trim() === '') {
      errors.push('Every surface must have a stable non-empty id.');
      continue;
    }
    if (surfaceIds.has(surface.id)) errors.push(`Duplicate surface id: ${surface.id}`);
    surfaceIds.add(surface.id);

    if (!allowedStatuses.has(surface.status)) {
      errors.push(`${surface.id} has unsupported status ${JSON.stringify(surface.status)}.`);
    }
    if (typeof surface.owner !== 'string' || surface.owner.trim() === '') {
      errors.push(`${surface.id} must define an owner.`);
    }

    const routeNames = requireStringArray(surface, 'route_names');
    requireStringArray(surface, 'roles');
    requireStringArray(surface, 'states');
    requireStringArray(surface, 'viewports');
    requireStringArray(surface, 'browsers');
    requireStringArray(surface, 'evidence_layers');

    for (const routeName of routeNames) {
      const currentOwner = routeOwners.get(routeName);
      if (currentOwner) {
        errors.push(`Route ${routeName} is classified by both ${currentOwner} and ${surface.id}.`);
      } else {
        routeOwners.set(routeName, surface.id);
      }
    }

    const gaps = Array.isArray(surface.gaps) ? surface.gaps : [];
    if (!Array.isArray(surface.gaps)) errors.push(`${surface.id} must define gaps as an array.`);
    if (surface.status === 'covered' && gaps.length > 0) {
      errors.push(`${surface.id} is covered but still declares gaps.`);
    }
    if ((surface.status === 'partial' || surface.status === 'planned') && gaps.length === 0) {
      errors.push(`${surface.id} is ${surface.status} but does not identify an exact gap.`);
    }
    if (strict && (surface.status === 'partial' || surface.status === 'planned')) {
      errors.push(`${surface.id} remains ${surface.status}; strict delivered-surface coverage cannot pass.`);
    }

    if (!Array.isArray(surface.evidence) || surface.evidence.length === 0) {
      errors.push(`${surface.id} must reference at least one evidence file.`);
      continue;
    }

    let stableMarkerCount = 0;
    for (const evidence of surface.evidence) {
      if (typeof evidence?.file !== 'string' || evidence.file.trim() === '') {
        errors.push(`${surface.id} contains evidence without a file path.`);
        continue;
      }
      const absolute = path.resolve(repoRoot, evidence.file);
      if (!absolute.startsWith(`${repoRoot}${path.sep}`)) {
        errors.push(`${surface.id} evidence escapes repository root: ${evidence.file}`);
        continue;
      }
      if (!fs.existsSync(absolute) || !fs.statSync(absolute).isFile()) {
        errors.push(`${surface.id} references missing evidence file ${evidence.file}.`);
        continue;
      }

      const markers = Array.isArray(evidence.markers) ? evidence.markers : [];
      if (!Array.isArray(evidence.markers)) {
        errors.push(`${surface.id} evidence ${evidence.file} must define markers as an array.`);
        continue;
      }
      const content = fs.readFileSync(absolute, 'utf8');
      for (const marker of markers) {
        if (typeof marker !== 'string' || marker.trim() === '') {
          errors.push(`${surface.id} evidence ${evidence.file} contains an empty marker.`);
          continue;
        }
        stableMarkerCount += 1;
        if (!content.includes(marker)) {
          errors.push(`${surface.id} evidence marker not found in ${evidence.file}: ${marker}`);
        }
      }
    }

    if (surface.status === 'covered' && stableMarkerCount === 0) {
      errors.push(`${surface.id} is covered but has no stable evidence marker.`);
    }
  }

  const exclusions = Array.isArray(manifest.route_name_exclusions) ? manifest.route_name_exclusions : [];
  for (const [index, exclusion] of exclusions.entries()) {
    const hasSelector = typeof exclusion?.exact === 'string' || typeof exclusion?.prefix === 'string';
    if (!hasSelector) errors.push(`route_name_exclusions[${index}] needs exact or prefix.`);
    if (typeof exclusion?.reason !== 'string' || exclusion.reason.trim() === '') {
      errors.push(`route_name_exclusions[${index}] needs a bounded reason.`);
    }
  }

  let discoveredNamedRouteCount = null;
  let classifiedNamedRouteCount = routeOwners.size;
  if (!manifestOnly && errors.length === 0) {
    let routeList;
    try {
      const output = execFileSync('php', ['artisan', 'route:list', '--json'], {
        cwd: repoRoot,
        env: process.env,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      routeList = JSON.parse(output);
    } catch (error) {
      errors.push(`Cannot obtain Laravel route list: ${error.stderr?.toString().trim() || error.message}`);
      routeList = [];
    }

    const allNamedRoutes = new Set();
    const classifiedRuntimeRoutes = new Set();
    for (const route of Array.isArray(routeList) ? routeList : []) {
      const name = typeof route?.name === 'string' ? route.name.trim() : '';
      if (name === '') continue;
      allNamedRoutes.add(name);
      if (exclusions.some((exclusion) => exclusionMatches(name, exclusion))) continue;
      classifiedRuntimeRoutes.add(name);
    }

    discoveredNamedRouteCount = allNamedRoutes.size;
    classifiedNamedRouteCount = classifiedRuntimeRoutes.size;

    for (const name of [...classifiedRuntimeRoutes].sort()) {
      if (!routeOwners.has(name)) errors.push(`Named Laravel route is not classified: ${name}`);
    }
    for (const [name, owner] of [...routeOwners.entries()].sort(([left], [right]) => left.localeCompare(right))) {
      if (!classifiedRuntimeRoutes.has(name)) {
        errors.push(`Manifest route does not exist in the exact application route table: ${name} (${owner})`);
      }
    }

    for (const exclusion of exclusions) {
      const matched = [...allNamedRoutes].some((name) => exclusionMatches(name, exclusion));
      if (!matched) warnings.push(`Route exclusion currently matches no named route: ${JSON.stringify(exclusion)}`);
    }
  }

  const statusCounts = {};
  for (const surface of manifest.surfaces ?? []) {
    statusCounts[surface.status] = (statusCounts[surface.status] ?? 0) + 1;
  }

  const report = {
    schema_version: manifest.schema_version,
    strict,
    manifest_only: manifestOnly,
    surface_count: manifest.surfaces?.length ?? 0,
    status_counts: statusCounts,
    manifest_route_count: routeOwners.size,
    discovered_named_route_count: discoveredNamedRouteCount,
    classified_runtime_route_count: classifiedNamedRouteCount,
    errors,
    warnings,
  };

  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (errors.length > 0) process.exitCode = 1;
}
