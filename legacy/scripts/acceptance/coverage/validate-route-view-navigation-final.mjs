import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import {
  loadRepositoryInputs as loadCompleteInputs,
  validateRouteViewNavigationInventory,
} from './validate-route-view-navigation-complete.mjs';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(coverageRoot, '../../..');

function readPolicy(repoRoot, file) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, 'docs/testing', file), 'utf8'));
}

export function loadRepositoryInputs(repoRoot = defaultRepoRoot) {
  const inputs = loadCompleteInputs(repoRoot);
  const exceptions = readPolicy(repoRoot, 'ROUTE_VIEW_NAVIGATION_ENDPOINT_EXCEPTIONS.json');
  if (exceptions.schema_version !== 1 || !Array.isArray(exceptions.route_overrides)) {
    throw new Error('Route/view/navigation endpoint exceptions must use schema_version 1 and route_overrides.');
  }
  inputs.contract.route_overrides = [
    ...(inputs.contract.route_overrides ?? []),
    ...exceptions.route_overrides,
  ];

  const delegated = readPolicy(repoRoot, 'ROUTE_VIEW_NAVIGATION_DELEGATED_BINDINGS.json');
  if (delegated.schema_version !== 1 || !Array.isArray(delegated.bindings)) {
    throw new Error('Delegated route/view bindings must use schema_version 1 and bindings.');
  }
  for (const entry of delegated.bindings) {
    const binding = inputs.routeBindings.get(entry.route_name);
    if (!binding) throw new Error(`Delegated binding references unknown route ${entry.route_name}.`);
    if (!binding.views.some((view) => view.name === entry.view)) {
      binding.views.push({ name: entry.view, source: entry.source });
    }
  }
  return inputs;
}

export { validateRouteViewNavigationInventory };

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
