import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(coverageRoot, '../../..');
const manifestPath = path.join(coverageRoot, 'portal-coverage-manifest.json');
const defaultEvidencePath = path.join(repoRoot, 'docs/testing/PORTAL_ROUTE_VIEW_NAVIGATION_EVIDENCE.json');
const surfaceRoot = path.join(coverageRoot, 'surfaces');
const inspectorPath = path.join(coverageRoot, 'inspect-route-view-navigation.php');
const originalReadFileSync = fs.readFileSync.bind(fs);

function evidencePathFromArguments() {
  const argument = process.argv.slice(2).find((entry) => entry.startsWith('--evidence='));
  if (!argument) return defaultEvidencePath;
  return path.resolve(process.cwd(), argument.slice('--evidence='.length).trim());
}

function asReadResult(content, options) {
  if (typeof options === 'string' || options?.encoding) return content;
  return Buffer.from(content);
}

const evidencePath = evidencePathFromArguments();
const manifest = JSON.parse(originalReadFileSync(manifestPath, 'utf8'));
const surfaces = [...(Array.isArray(manifest.surfaces) ? manifest.surfaces : [])];
if (fs.existsSync(surfaceRoot)) {
  for (const entry of fs.readdirSync(surfaceRoot, { withFileTypes: true })
    .filter((candidate) => candidate.isFile() && candidate.name.endsWith('.json'))
    .sort((left, right) => left.name.localeCompare(right.name))) {
    const fragment = JSON.parse(originalReadFileSync(path.join(surfaceRoot, entry.name), 'utf8'));
    const fragmentSurfaces = Array.isArray(fragment) ? fragment : fragment.surfaces;
    if (!Array.isArray(fragmentSurfaces) || fragmentSurfaces.length === 0) {
      throw new Error(`Surface fragment ${entry.name} must contain a non-empty surface array`);
    }
    surfaces.push(...fragmentSurfaces);
  }
}
const hydratedManifest = `${JSON.stringify({ ...manifest, surfaces })}\n`;

const evidence = JSON.parse(originalReadFileSync(evidencePath, 'utf8'));
const inspection = JSON.parse(execFileSync('php', [inspectorPath], {
  cwd: repoRoot,
  env: process.env,
  encoding: 'utf8',
  stdio: ['ignore', 'pipe', 'pipe'],
}));
const routes = new Map((inspection.routes ?? []).map((route) => [route.name, route]));
const expandedDirectEntries = [...(Array.isArray(evidence.direct_entry_routes) ? evidence.direct_entry_routes : [])];

for (const [index, pattern] of (evidence.direct_entry_route_patterns ?? []).entries()) {
  const marker = `direct_entry_route_patterns[${index}]`;
  if (typeof pattern?.prefix !== 'string' || pattern.prefix === '') throw new Error(`${marker} needs a non-empty prefix`);
  if (pattern.canonical_route !== 'strip_prefix') throw new Error(`${marker} has unsupported canonical_route`);
  if (!Array.isArray(pattern.routes) || pattern.routes.length === 0) throw new Error(`${marker} needs an exact non-empty routes array`);
  if (typeof pattern.rationale !== 'string' || pattern.rationale.length < 20) throw new Error(`${marker} needs a bounded rationale`);
  if (!Array.isArray(pattern.evidence) || pattern.evidence.length === 0) throw new Error(`${marker} needs evidence`);

  for (const routeName of pattern.routes) {
    if (typeof routeName !== 'string' || !routeName.startsWith(pattern.prefix)) throw new Error(`${marker} contains invalid route ${String(routeName)}`);
    const canonicalName = routeName.slice(pattern.prefix.length);
    const route = routes.get(routeName);
    const canonical = routes.get(canonicalName);
    if (!route || !canonical) throw new Error(`${marker} cannot bind ${routeName} to ${canonicalName}`);
    if (pattern.require_same_action && route.action !== canonical.action) throw new Error(`${routeName} and ${canonicalName} do not share an action`);
    const routeViews = [...new Set((route.views ?? []).map((view) => view.name))].sort();
    const canonicalViews = [...new Set((canonical.views ?? []).map((view) => view.name))].sort();
    if (pattern.require_same_views && JSON.stringify(routeViews) !== JSON.stringify(canonicalViews)) {
      throw new Error(`${routeName} and ${canonicalName} do not share rendered views`);
    }
    expandedDirectEntries.push({
      route: routeName,
      rationale: `${pattern.rationale} Canonical route: ${canonicalName}.`,
      evidence: [...pattern.evidence, `runtime alias binding: ${routeName} -> ${canonicalName}`],
    });
  }
}
const hydratedEvidence = `${JSON.stringify({ ...evidence, direct_entry_routes: expandedDirectEntries })}\n`;

fs.readFileSync = (file, options) => {
  const absolute = path.resolve(String(file));
  if (absolute === manifestPath) return asReadResult(hydratedManifest, options);
  if (absolute === evidencePath) return asReadResult(hydratedEvidence, options);
  return originalReadFileSync(file, options);
};

await import('./validate-route-view-navigation.mjs');
