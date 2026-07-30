import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const manifestPath = path.join(coverageRoot, 'portal-coverage-manifest.json');
const surfaceRoot = path.join(coverageRoot, 'surfaces');
const originalReadFileSync = fs.readFileSync.bind(fs);
const manifest = JSON.parse(originalReadFileSync(manifestPath, 'utf8'));
const surfaces = [...(Array.isArray(manifest.surfaces) ? manifest.surfaces : [])];

if (fs.existsSync(surfaceRoot)) {
  for (const entry of fs.readdirSync(surfaceRoot, { withFileTypes: true })
    .filter((candidate) => candidate.isFile() && candidate.name.endsWith('.json'))
    .sort((left, right) => left.name.localeCompare(right.name))) {
    const fragment = JSON.parse(originalReadFileSync(path.join(surfaceRoot, entry.name), 'utf8'));
    const fragmentSurfaces = Array.isArray(fragment) ? fragment : fragment.surfaces;
    if (Array.isArray(fragmentSurfaces)) surfaces.push(...fragmentSurfaces);
  }
}

const hydratedManifest = `${JSON.stringify({ ...manifest, surfaces })}\n`;
fs.readFileSync = (file, options) => {
  if (path.resolve(String(file)) !== manifestPath) return originalReadFileSync(file, options);
  if (typeof options === 'string' || options?.encoding) return hydratedManifest;
  return Buffer.from(hydratedManifest);
};

await import('./validate-route-view-navigation.mjs');
