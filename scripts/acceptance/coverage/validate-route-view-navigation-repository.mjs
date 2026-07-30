import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { validateRouteViewNavigationInventory } from './validate-route-view-navigation-inventory.mjs';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(coverageRoot, '../../..');

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() !== '';
}

function selectorMatches(value, selector) {
  if (nonEmptyString(selector?.exact)) return value === selector.exact;
  if (nonEmptyString(selector?.prefix)) return value.startsWith(selector.prefix);
  return false;
}

function walkFiles(root, predicate = () => true) {
  if (!fs.existsSync(root)) return [];

  const files = [];
  const visit = (directory) => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })
      .sort((left, right) => left.name.localeCompare(right.name))) {
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) visit(absolute);
      if (entry.isFile() && predicate(absolute)) files.push(absolute);
    }
  };

  visit(root);
  return files;
}

function relativeFile(repoRoot, absolute) {
  return path.relative(repoRoot, absolute).split(path.sep).join('/');
}

function readSurfaceFragments(root) {
  return walkFiles(root, (file) => file.endsWith('.json')).flatMap((file) => {
    const value = readJson(file);
    return Array.isArray(value) ? value : (Array.isArray(value.surfaces) ? value.surfaces : []);
  });
}

function routeNameReferences(content, file, kind) {
  const references = [];
  const patterns = [
    /\broute\(\s*(['"])([^'"]+)\1/g,
    /['"]route['"]\s*=>\s*(['"])([^'"]+)\1/g,
  ];

  for (const pattern of patterns) {
    for (const match of content.matchAll(pattern)) {
      references.push({
        route_name: match[2],
        kind,
        source: { file, markers: [match[0]] },
      });
    }
  }

  return references;
}

function bladeStructuralReferences(content) {
  const names = [];
  const pattern = /@(extends|include|includeIf|includeWhen|includeUnless|component|each)\(\s*(['"])([^'"]+)\2/g;
  for (const match of content.matchAll(pattern)) names.push(match[3]);
  return names;
}

function routeSourceHints(repoRoot) {
  const hints = new Map();
  const routeRoot = path.join(repoRoot, 'routes');

  for (const absolute of walkFiles(routeRoot, (file) => file.endsWith('.php'))) {
    const file = relativeFile(repoRoot, absolute);
    const content = fs.readFileSync(absolute, 'utf8');
    const namePattern = /->name\(\s*(['"])([^'"]+)\1\s*\)/g;

    for (const match of content.matchAll(namePattern)) {
      const routeName = match[2];
      const start = content.lastIndexOf('Route::', match.index);
      const end = content.indexOf(';', match.index);
      if (start < 0 || end < 0) continue;

      const statement = content.slice(start, end + 1);
      const viewMatches = [
        ...statement.matchAll(/Route::view\(\s*(['"])[^'"]+\1\s*,\s*(['"])([^'"]+)\2/g),
        ...statement.matchAll(/(?<!::)\bview\(\s*(['"])([^'"]+)\1/g),
      ];
      const views = [];
      for (const viewMatch of viewMatches) {
        const name = viewMatch.length >= 4 ? viewMatch[3] : viewMatch[2];
        if (!views.some((entry) => entry.name === name)) {
          views.push({
            name,
            source: { file, markers: [viewMatch[0], match[0]] },
          });
        }
      }

      const redirect = /Route::redirect\(|\bredirect\(|\bto_route\(/.test(statement)
        ? { file, markers: [match[0]] }
        : null;

      hints.set(routeName, { views, redirect });
    }
  }

  return hints;
}

function extractMethodSource(content, method) {
  const escaped = method.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const pattern = new RegExp(`function\\s+${escaped}\\s*\\(`, 'u');
  const match = pattern.exec(content);
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
      if (escapedChar) {
        escapedChar = false;
        continue;
      }
      if (current === '\\') {
        escapedChar = true;
        continue;
      }
      if (current === quote) quote = null;
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

function controllerBinding(repoRoot, action) {
  if (!nonEmptyString(action) || action === 'Closure') return { views: [], redirect: null };

  const [className, explicitMethod] = action.split('@');
  if (!className.startsWith('App\\')) return { views: [], redirect: null };

  const method = explicitMethod || '__invoke';
  const relative = `${className.replace(/^App\\/u, 'app\\').replaceAll('\\', '/')}.php`;
  const absolute = path.join(repoRoot, relative);
  if (!fs.existsSync(absolute)) return { views: [], redirect: null };

  const content = fs.readFileSync(absolute, 'utf8');
  const methodSource = extractMethodSource(content, method);
  if (methodSource === null) return { views: [], redirect: null };

  const views = [];
  const viewPattern = /(?:\bview|View::make|response\(\)->view)\(\s*(['"])([^'"]+)\1/g;
  for (const match of methodSource.matchAll(viewPattern)) {
    if (!views.some((entry) => entry.name === match[2])) {
      views.push({
        name: match[2],
        source: {
          file: relative,
          markers: [match[0], `function ${method}`],
        },
      });
    }
  }

  const redirect = /\bredirect\s*\(|\bto_route\s*\(|->route\s*\(/.test(methodSource)
    ? { file: relative, markers: [`function ${method}`] }
    : null;

  return { views, redirect };
}

function discoverRouteBindings(repoRoot, runtimeRoutes) {
  const hints = routeSourceHints(repoRoot);
  const bindings = new Map();

  for (const route of runtimeRoutes) {
    const name = nonEmptyString(route?.name) ? route.name.trim() : '';
    if (name === '') continue;

    const hint = hints.get(name) ?? { views: [], redirect: null };
    const controller = controllerBinding(repoRoot, typeof route?.action === 'string' ? route.action.trim() : '');
    const views = [...hint.views];
    for (const view of controller.views) {
      if (!views.some((entry) => entry.name === view.name)) views.push(view);
    }

    bindings.set(name, {
      views,
      redirect: hint.redirect ?? controller.redirect,
    });
  }

  return bindings;
}

function discoverViews(repoRoot, routeNameExclusions) {
  const viewRoot = path.join(repoRoot, 'resources/views');
  const views = new Map();
  const structuralReferences = new Set();
  const navigationReferences = [];

  for (const absolute of walkFiles(viewRoot, (file) => file.endsWith('.blade.php'))) {
    const file = relativeFile(repoRoot, absolute);
    const content = fs.readFileSync(absolute, 'utf8');
    const name = path.relative(viewRoot, absolute)
      .replace(/\.blade\.php$/u, '')
      .split(path.sep)
      .join('.');
    views.set(name, { name, file, content });
    for (const structural of bladeStructuralReferences(content)) structuralReferences.add(structural);
    navigationReferences.push(...routeNameReferences(content, file, 'contextual'));
  }

  const navigationRoot = path.join(repoRoot, 'resources/navigation');
  for (const absolute of walkFiles(navigationRoot, (file) => file.endsWith('.php'))) {
    const file = relativeFile(repoRoot, absolute);
    const content = fs.readFileSync(absolute, 'utf8');
    navigationReferences.push(...routeNameReferences(content, file, 'global'));
  }

  return {
    views,
    structuralReferences,
    navigationReferences: navigationReferences.filter((reference) => (
      !routeNameExclusions.some((exclusion) => selectorMatches(reference.route_name, exclusion))
    )),
  };
}

function runtimeRouteList(repoRoot) {
  const output = execFileSync('php', ['artisan', 'route:list', '--json'], {
    cwd: repoRoot,
    env: process.env,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  return JSON.parse(output);
}

export function loadRepositoryInputs(repoRoot = defaultRepoRoot) {
  const manifest = readJson(path.join(repoRoot, 'scripts/acceptance/coverage/portal-coverage-manifest.json'));
  const manifestSurfaces = [
    ...(Array.isArray(manifest.surfaces) ? manifest.surfaces : []),
    ...readSurfaceFragments(path.join(repoRoot, 'scripts/acceptance/coverage/surfaces')),
  ];
  const routeNameExclusions = Array.isArray(manifest.route_name_exclusions) ? manifest.route_name_exclusions : [];
  const runtimeRoutes = runtimeRouteList(repoRoot);
  const routeBindings = discoverRouteBindings(repoRoot, runtimeRoutes);
  const discovered = discoverViews(repoRoot, routeNameExclusions);

  return {
    contract: readJson(path.join(repoRoot, 'docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json')),
    manifestSurfaces,
    routeNameExclusions,
    runtimeRoutes,
    routeBindings,
    views: discovered.views,
    structuralReferences: discovered.structuralReferences,
    navigationReferences: discovered.navigationReferences,
    repoRoot,
  };
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
