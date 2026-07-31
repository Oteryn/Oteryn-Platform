import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { loadRepositoryInputs as loadBaseInputs } from './validate-route-view-navigation-repository.mjs';
import { validateRouteViewNavigationInventory as validateCore } from './validate-route-view-navigation-inventory.mjs';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(coverageRoot, '../../..');

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() !== '';
}

function routeMethods(route) {
  const raw = Array.isArray(route?.methods) ? route.methods.join('|') : (route?.method ?? route?.methods ?? '');
  return String(raw).split('|').map((method) => method.trim().toUpperCase()).filter(Boolean);
}

function hasReadMethod(route) {
  const methods = routeMethods(route);
  return methods.includes('GET') || methods.includes('HEAD');
}

function extractMethodSource(content, method) {
  const escaped = method.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`(?:public|protected|private)?\\s*(?:static\\s+)?function\\s+${escaped}\\s*\\(`, 'u').exec(content);
  if (!match) return null;
  const opening = content.indexOf('{', match.index + match[0].length);
  if (opening < 0) return null;

  let depth = 0;
  let quote = null;
  let escapedChar = false;
  let lineComment = false;
  let blockComment = false;
  for (let index = opening; index < content.length; index += 1) {
    const current = content[index];
    const next = content[index + 1] ?? '';
    if (lineComment) {
      if (current === '\n') lineComment = false;
      continue;
    }
    if (blockComment) {
      if (current === '*' && next === '/') {
        blockComment = false;
        index += 1;
      }
      continue;
    }
    if (quote !== null) {
      if (escapedChar) escapedChar = false;
      else if (current === '\\') escapedChar = true;
      else if (current === quote) quote = null;
      continue;
    }
    if (current === '/' && next === '/') {
      lineComment = true;
      index += 1;
      continue;
    }
    if (current === '/' && next === '*') {
      blockComment = true;
      index += 1;
      continue;
    }
    if (current === "'" || current === '"') {
      quote = current;
      continue;
    }
    if (current === '{') depth += 1;
    if (current === '}') {
      depth -= 1;
      if (depth === 0) return content.slice(match.index, index + 1);
    }
  }
  return null;
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

function recursiveControllerBinding(repoRoot, action) {
  if (!nonEmptyString(action) || action === 'Closure') return { views: [], redirect: null };
  const [className, explicitMethod] = action.split('@');
  if (!className.startsWith('App\\')) return { views: [], redirect: null };
  const relative = `${className.replace(/^App\\/u, 'app\\').replaceAll('\\', '/')}.php`;
  const absolute = path.join(repoRoot, relative);
  if (!fs.existsSync(absolute)) return { views: [], redirect: null };

  const content = fs.readFileSync(absolute, 'utf8');
  const queue = [explicitMethod || '__invoke'];
  const visited = new Set();
  const views = [];
  let redirect = null;

  while (queue.length > 0) {
    const method = queue.shift();
    if (visited.has(method)) continue;
    visited.add(method);
    const source = extractMethodSource(content, method);
    if (source === null) continue;

    const viewPattern = /(?:\bview|View::make|response\(\)->view)\(\s*(['"])([^'"]+)\1/g;
    for (const match of source.matchAll(viewPattern)) {
      if (!views.some((entry) => entry.name === match[2])) {
        views.push({
          name: match[2],
          source: { file: relative, markers: [match[0], `function ${method}`] },
        });
      }
    }
    if (redirect === null && /\bredirect\s*\(|\bto_route\s*\(|->route\s*\(/.test(source)) {
      redirect = { file: relative, markers: [`function ${method}`] };
    }
    for (const match of source.matchAll(/\$this->([A-Za-z_][A-Za-z0-9_]*)\s*\(/g)) {
      if (!visited.has(match[1])) queue.push(match[1]);
    }
  }

  return { views, redirect };
}

function enrichBindings(inputs) {
  for (const route of inputs.runtimeRoutes) {
    const name = nonEmptyString(route?.name) ? route.name.trim() : '';
    if (name === '') continue;
    const discovered = recursiveControllerBinding(inputs.repoRoot, typeof route?.action === 'string' ? route.action : '');
    const current = inputs.routeBindings.get(name) ?? { views: [], redirect: null };
    for (const view of discovered.views) {
      if (!current.views.some((entry) => entry.name === view.name)) current.views.push(view);
    }
    current.redirect ??= discovered.redirect;
    inputs.routeBindings.set(name, current);
  }
  return inputs;
}

export function loadRepositoryInputs(repoRoot = defaultRepoRoot) {
  return enrichBindings(loadBaseInputs(repoRoot));
}

export function validateRouteViewNavigationInventory(inputs) {
  const actionViews = new Set();
  const runtimeByName = new Map(inputs.runtimeRoutes.map((route) => [route.name, route]));
  for (const [routeName, binding] of inputs.routeBindings.entries()) {
    const route = runtimeByName.get(routeName);
    if (!route || hasReadMethod(route)) continue;
    for (const entry of binding.views ?? []) actionViews.add(entry.name);
  }

  const coreInputs = {
    ...inputs,
    structuralReferences: new Set([...(inputs.structuralReferences ?? []), ...actionViews]),
  };
  const report = validateCore(coreInputs);
  const boundNames = new Set();
  for (const record of report.inventory) {
    const binding = inputs.routeBindings.get(record.route_name) ?? { views: [] };
    if (record.kind !== 'rendered_screen' && (binding.views ?? []).length > 0) {
      for (const entry of binding.views) {
        if (!inputs.views.has(entry.name)) {
          report.errors.push(`Action route ${record.route_name} references missing Blade view ${entry.name}.`);
        } else {
          boundNames.add(entry.name);
        }
        sourceMarker(inputs.repoRoot, entry.source, `${record.route_name} -> ${entry.name}`, report.errors);
        record.views.push({ name: entry.name, source: entry.source });
      }
    }
    for (const entry of record.views) boundNames.add(entry.name);
  }

  report.bound_view_count = boundNames.size;
  const actionOnly = [...actionViews].filter((name) => boundNames.has(name));
  report.structural_view_count = Math.max(0, report.structural_view_count - actionOnly.length);
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
