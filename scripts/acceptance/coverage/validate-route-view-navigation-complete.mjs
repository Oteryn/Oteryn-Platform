import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import {
  loadRepositoryInputs,
  validateRouteViewNavigationInventory as validateDelegated,
} from './validate-route-view-navigation-contract.mjs';

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() !== '';
}

function sourceMarker(repoRoot, source, owner, errors) {
  if (!nonEmptyString(source?.file) || !Array.isArray(source?.markers) || source.markers.length === 0) {
    errors.push(`${owner} must define a repository-relative source file and exact markers.`);
    return;
  }
  const absolute = path.resolve(repoRoot, source.file);
  if (!absolute.startsWith(`${repoRoot}${path.sep}`) || !fs.existsSync(absolute)) {
    errors.push(`${owner} references missing or escaping source file ${source.file}.`);
    return;
  }
  const content = fs.readFileSync(absolute, 'utf8');
  for (const marker of source.markers) {
    if (!nonEmptyString(marker) || !content.includes(marker)) {
      errors.push(`${owner} source marker not found in ${source.file}: ${marker}`);
    }
  }
}

export { loadRepositoryInputs };

export function validateRouteViewNavigationInventory(inputs) {
  const allBoundViewNames = new Set();
  for (const binding of inputs.routeBindings.values()) {
    for (const entry of binding.views ?? []) allBoundViewNames.add(entry.name);
  }

  const report = validateDelegated({
    ...inputs,
    structuralReferences: new Set([...(inputs.structuralReferences ?? []), ...allBoundViewNames]),
  });

  const recordByRoute = new Map(report.inventory.map((record) => [record.route_name, record]));
  const previouslyBound = new Set(report.inventory.flatMap((record) => record.views.map((entry) => entry.name)));
  const newlyBound = new Set();

  for (const [routeName, binding] of inputs.routeBindings.entries()) {
    const record = recordByRoute.get(routeName);
    if (!record || record.kind === 'rendered_screen') continue;
    for (const entry of binding.views ?? []) {
      if (!inputs.views.has(entry.name)) {
        report.errors.push(`Route ${routeName} references missing Blade view ${entry.name}.`);
      }
      sourceMarker(inputs.repoRoot, entry.source, `${routeName} -> ${entry.name}`, report.errors);
      if (!record.views.some((existing) => existing.name === entry.name)) {
        record.views.push({ name: entry.name, source: entry.source });
      }
      if (!previouslyBound.has(entry.name)) newlyBound.add(entry.name);
    }
  }

  const finalBound = new Set(report.inventory.flatMap((record) => record.views.map((entry) => entry.name)));
  report.bound_view_count = finalBound.size;
  report.structural_view_count = Math.max(0, report.structural_view_count - newlyBound.size);
  return report;
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  let report;
  try {
    report = validateRouteViewNavigationInventory(loadRepositoryInputs());
  } catch (error) {
    report = {
      schema_version: null,
      strict_closure: false,
      portal_surface_count: 0,
      runtime_route_count: 0,
      classified_route_count: 0,
      route_kind_counts: {},
      rendered_route_count: 0,
      bound_view_count: 0,
      blade_view_count: 0,
      structural_view_count: 0,
      excluded_view_count: 0,
      orphan_view_count: 0,
      navigation_reference_count: 0,
      direct_entry_route_count: 0,
      inventory: [],
      errors: [`Cannot validate route/view/navigation inventory: ${error.stderr?.toString().trim() || error.message}`],
      warnings: [],
    };
  }
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.errors.length > 0) process.exitCode = 1;
}
