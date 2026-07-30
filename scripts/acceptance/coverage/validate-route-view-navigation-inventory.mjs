import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(coverageRoot, '../../..');
const allowedKinds = [
  'rendered_screen',
  'form_action',
  'redirect',
  'resource_supporting',
  'exception',
];

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() !== '';
}

function stringArray(value) {
  return Array.isArray(value) && value.length > 0 && value.every(nonEmptyString);
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

function selectorMatches(value, selector) {
  if (nonEmptyString(selector?.exact)) return value === selector.exact;
  if (nonEmptyString(selector?.prefix)) return value.startsWith(selector.prefix);
  return false;
}

function exclusionMatches(name, exclusion) {
  return selectorMatches(name, exclusion);
}

function sourceMarker(repoRoot, source, owner, errors) {
  if (!nonEmptyString(source?.file)) {
    errors.push(`${owner} must define a repository-relative source file.`);
    return;
  }
  if (!stringArray(source?.markers)) {
    errors.push(`${owner} must define at least one exact source marker.`);
    return;
  }

  const absolute = path.resolve(repoRoot, source.file);
  if (!absolute.startsWith(`${repoRoot}${path.sep}`)) {
    errors.push(`${owner} source escapes repository root: ${source.file}`);
    return;
  }
  if (!fs.existsSync(absolute) || !fs.statSync(absolute).isFile()) {
    errors.push(`${owner} references missing source file ${source.file}.`);
    return;
  }

  const content = fs.readFileSync(absolute, 'utf8');
  for (const marker of source.markers) {
    if (!content.includes(marker)) {
      errors.push(`${owner} source marker not found in ${source.file}: ${marker}`);
    }
  }
}

function routeMethods(route) {
  const raw = Array.isArray(route?.methods)
    ? route.methods.join('|')
    : (route?.method ?? route?.methods ?? '');
  return String(raw)
    .split('|')
    .map((method) => method.trim().toUpperCase())
    .filter(Boolean);
}

function hasReadMethod(route) {
  const methods = routeMethods(route);
  return methods.includes('GET') || methods.includes('HEAD');
}

function routeAction(route) {
  return typeof route?.action === 'string' ? route.action.trim() : '';
}

function viewNameFromFile(viewRoot, absolute) {
  return path.relative(viewRoot, absolute)
    .replace(/\.blade\.php$/u, '')
    .split(path.sep)
    .join('.');
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
        source: {
          file,
          markers: [match[0]],
        },
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
        ...statement.matchAll(/\bview\(\s*(['"])([^'"]+)\1/g),
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

      hints.set(routeName, { views, redirect, source: { file, markers: [match[0]] } });
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
  if (!className.startsWith('App\\Http\\Controllers\\')) return { views: [], redirect: null };

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
    const controller = controllerBinding(repoRoot, routeAction(route));
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

function discoverViews(repoRoot) {
  const viewRoot = path.join(repoRoot, 'resources/views');
  const views = new Map();
  const structuralReferences = new Set();
  const navigationReferences = [];

  for (const absolute of walkFiles(viewRoot, (file) => file.endsWith('.blade.php'))) {
    const file = relativeFile(repoRoot, absolute);
    const content = fs.readFileSync(absolute, 'utf8');
    const name = viewNameFromFile(viewRoot, absolute);
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

  return { views, structuralReferences, navigationReferences };
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

function viewExcluded(view, contract) {
  const relativeSegments = view.file.split('/');
  const basename = relativeSegments.at(-1);
  if ((contract.structural_view_segments ?? []).some((segment) => relativeSegments.includes(segment))) return true;
  if ((contract.structural_view_basenames ?? []).includes(basename)) return true;
  if (view.content.includes('@props(')) return true;
  return false;
}

function findOverride(routeName, contract) {
  return (contract.route_overrides ?? []).find((entry) => selectorMatches(routeName, entry));
}

function findDirectEntry(routeName, contract) {
  return (contract.direct_entry_routes ?? []).find((entry) => selectorMatches(routeName, entry));
}

export function validateRouteViewNavigationInventory({
  contract,
  manifestSurfaces,
  routeNameExclusions = [],
  runtimeRoutes,
  routeBindings,
  views,
  structuralReferences,
  navigationReferences,
  repoRoot = defaultRepoRoot,
}) {
  const errors = [];
  const warnings = [];

  if (contract?.schema_version !== 1) errors.push('route/view/navigation inventory schema_version must be 1.');
  if (contract?.issue !== 360) errors.push('route/view/navigation inventory issue must be 360.');
  if (contract?.parent_issue !== 326) errors.push('route/view/navigation inventory parent_issue must be 326.');
  if (contract?.strict_closure !== true) errors.push('route/view/navigation inventory strict_closure must be true.');
  if (JSON.stringify(contract?.route_kinds) !== JSON.stringify(allowedKinds)) {
    errors.push(`route_kinds must be exactly ${JSON.stringify(allowedKinds)}.`);
  }
  if (!Array.isArray(contract?.nonclaims) || contract.nonclaims.length < 3 || !contract.nonclaims.every(nonEmptyString)) {
    errors.push('route/view/navigation inventory must preserve at least three explicit nonclaims.');
  }
  if (!Array.isArray(contract?.structural_view_segments) || !contract.structural_view_segments.every(nonEmptyString)) {
    errors.push('structural_view_segments must be an array of non-empty strings.');
  }
  if (!Array.isArray(contract?.structural_view_basenames) || !contract.structural_view_basenames.every(nonEmptyString)) {
    errors.push('structural_view_basenames must be an array of non-empty strings.');
  }

  const manifestByRoute = new Map();
  const manifestById = new Map();
  for (const surface of Array.isArray(manifestSurfaces) ? manifestSurfaces : []) {
    if (!nonEmptyString(surface?.id)) {
      errors.push('Every portal manifest surface must have a stable id.');
      continue;
    }
    if (manifestById.has(surface.id)) errors.push(`Duplicate portal manifest surface id: ${surface.id}`);
    manifestById.set(surface.id, surface);

    for (const routeName of Array.isArray(surface.route_names) ? surface.route_names : []) {
      if (!nonEmptyString(routeName)) continue;
      const existing = manifestByRoute.get(routeName);
      if (existing) {
        errors.push(`Route ${routeName} is owned by both ${existing.id} and ${surface.id}.`);
      } else {
        manifestByRoute.set(routeName, surface);
      }
    }
  }

  const runtimeByName = new Map();
  for (const route of Array.isArray(runtimeRoutes) ? runtimeRoutes : []) {
    const name = nonEmptyString(route?.name) ? route.name.trim() : '';
    if (name === '' || routeNameExclusions.some((exclusion) => exclusionMatches(name, exclusion))) continue;
    if (runtimeByName.has(name)) errors.push(`Runtime route name is duplicated: ${name}`);
    runtimeByName.set(name, route);
  }

  for (const name of runtimeByName.keys()) {
    if (!manifestByRoute.has(name)) errors.push(`Named Laravel route is not owned by the portal manifest: ${name}`);
  }
  for (const name of manifestByRoute.keys()) {
    if (!runtimeByName.has(name)) errors.push(`Portal manifest route is absent from the exact runtime route table: ${name}`);
  }

  for (const [index, entry] of (contract.route_overrides ?? []).entries()) {
    if (!nonEmptyString(entry?.exact) && !nonEmptyString(entry?.prefix)) {
      errors.push(`route_overrides[${index}] requires exact or prefix.`);
    }
    if (!allowedKinds.includes(entry?.kind)) errors.push(`route_overrides[${index}] has unsupported kind.`);
    if (!nonEmptyString(entry?.rationale) || entry.rationale.trim().length < 80) {
      errors.push(`route_overrides[${index}] requires a bounded rationale of at least 80 characters.`);
    }
    if (![...runtimeByName.keys()].some((name) => selectorMatches(name, entry))) {
      errors.push(`route override matches no exact runtime route: ${JSON.stringify(entry)}`);
    }
  }

  for (const [index, entry] of (contract.direct_entry_routes ?? []).entries()) {
    if (!nonEmptyString(entry?.exact) && !nonEmptyString(entry?.prefix)) {
      errors.push(`direct_entry_routes[${index}] requires exact or prefix.`);
    }
    if (!nonEmptyString(entry?.rationale) || entry.rationale.trim().length < 80) {
      errors.push(`direct_entry_routes[${index}] requires a bounded rationale of at least 80 characters.`);
    }
  }

  for (const [index, entry] of (contract.view_exclusions ?? []).entries()) {
    if (!nonEmptyString(entry?.exact) && !nonEmptyString(entry?.prefix)) {
      errors.push(`view_exclusions[${index}] requires exact or prefix.`);
    }
    if (!nonEmptyString(entry?.rationale) || entry.rationale.trim().length < 80) {
      errors.push(`view_exclusions[${index}] requires a bounded rationale of at least 80 characters.`);
    }
  }

  const classified = [];
  const renderedRoutes = new Set();
  const boundViews = new Set();
  const kindCounts = Object.fromEntries(allowedKinds.map((kind) => [kind, 0]));

  for (const [name, route] of [...runtimeByName.entries()].sort(([left], [right]) => left.localeCompare(right))) {
    const surface = manifestByRoute.get(name);
    const binding = routeBindings.get(name) ?? { views: [], redirect: null };
    const override = findOverride(name, contract);
    let kind;
    let rationale = null;

    if (surface?.status === 'supporting_endpoint') {
      kind = 'resource_supporting';
    } else if (override) {
      kind = override.kind;
      rationale = override.rationale;
    } else if (!hasReadMethod(route)) {
      kind = 'form_action';
    } else if (binding.views.length > 0) {
      kind = 'rendered_screen';
    } else if (binding.redirect) {
      kind = 'redirect';
    } else {
      errors.push(`GET/HEAD route cannot be classified from exact source evidence: ${name} (${routeAction(route) || 'unknown action'}).`);
      continue;
    }

    kindCounts[kind] += 1;
    const record = {
      route_name: name,
      surface_id: surface?.id ?? null,
      kind,
      methods: routeMethods(route),
      uri: route?.uri ?? null,
      action: routeAction(route),
      views: [],
      rationale,
    };

    if (kind === 'rendered_screen') {
      const viewEntries = override?.views ?? binding.views;
      if (!Array.isArray(viewEntries) || viewEntries.length === 0) {
        errors.push(`Rendered route ${name} must bind at least one Blade page view.`);
      } else {
        renderedRoutes.add(name);
        for (const entry of viewEntries) {
          const viewName = typeof entry === 'string' ? entry : entry?.name;
          const source = typeof entry === 'string' ? override?.source : entry?.source;
          if (!nonEmptyString(viewName)) {
            errors.push(`Rendered route ${name} contains an empty view binding.`);
            continue;
          }
          if (!views.has(viewName)) {
            errors.push(`Rendered route ${name} references missing Blade view ${viewName}.`);
          } else {
            boundViews.add(viewName);
          }
          sourceMarker(repoRoot, source, `${name} -> ${viewName}`, errors);
          record.views.push({ name: viewName, source });
        }
      }
    } else if (kind === 'redirect') {
      const source = override?.source ?? binding.redirect;
      sourceMarker(repoRoot, source, `${name} redirect`, errors);
    } else if (kind === 'exception') {
      if (!override) errors.push(`${name} exception classification requires an explicit route override.`);
    }

    classified.push(record);
  }

  const navigationByRoute = new Map();
  for (const reference of Array.isArray(navigationReferences) ? navigationReferences : []) {
    if (!nonEmptyString(reference?.route_name)) {
      errors.push('Navigation reference contains an empty route name.');
      continue;
    }
    if (!runtimeByName.has(reference.route_name)) {
      errors.push(`Navigation source references unknown named route ${reference.route_name}.`);
    }
    sourceMarker(repoRoot, reference.source, `${reference.kind ?? 'navigation'} reference ${reference.route_name}`, errors);
    const entries = navigationByRoute.get(reference.route_name) ?? [];
    entries.push(reference);
    navigationByRoute.set(reference.route_name, entries);
  }

  let directEntryCount = 0;
  for (const routeName of renderedRoutes) {
    if ((navigationByRoute.get(routeName) ?? []).length > 0) continue;
    const direct = findDirectEntry(routeName, contract);
    if (!direct) {
      errors.push(`Rendered route has no global/contextual navigation reference or direct-entry rationale: ${routeName}`);
      continue;
    }
    directEntryCount += 1;
  }

  for (const entry of contract.direct_entry_routes ?? []) {
    const matched = [...renderedRoutes].filter((name) => selectorMatches(name, entry));
    if (matched.length === 0) errors.push(`Direct-entry exception matches no rendered route: ${JSON.stringify(entry)}`);
  }

  const structuralNames = new Set(structuralReferences ?? []);
  let orphanViewCount = 0;
  let structuralViewCount = 0;
  let excludedViewCount = 0;

  for (const view of views.values()) {
    if (boundViews.has(view.name)) continue;
    if (structuralNames.has(view.name) || viewExcluded(view, contract)) {
      structuralViewCount += 1;
      continue;
    }

    const exclusion = (contract.view_exclusions ?? []).find((entry) => selectorMatches(view.name, entry));
    if (exclusion) {
      excludedViewCount += 1;
      continue;
    }

    orphanViewCount += 1;
    errors.push(`Page-like Blade view is not bound to a delivered rendered route: ${view.name} (${view.file}).`);
  }

  for (const entry of contract.view_exclusions ?? []) {
    const matched = [...views.keys()].filter((name) => selectorMatches(name, entry));
    if (matched.length === 0) errors.push(`View exclusion matches no Blade view: ${JSON.stringify(entry)}`);
  }

  return {
    schema_version: contract?.schema_version ?? null,
    strict_closure: contract?.strict_closure === true,
    portal_surface_count: manifestById.size,
    runtime_route_count: runtimeByName.size,
    classified_route_count: classified.length,
    route_kind_counts: kindCounts,
    rendered_route_count: renderedRoutes.size,
    bound_view_count: boundViews.size,
    blade_view_count: views.size,
    structural_view_count: structuralViewCount,
    excluded_view_count: excludedViewCount,
    orphan_view_count: orphanViewCount,
    navigation_reference_count: navigationReferences.length,
    direct_entry_route_count: directEntryCount,
    inventory: classified,
    errors,
    warnings,
  };
}

export function loadRepositoryInputs(repoRoot = defaultRepoRoot) {
  const manifest = readJson(path.join(repoRoot, 'scripts/acceptance/coverage/portal-coverage-manifest.json'));
  const manifestSurfaces = [
    ...(Array.isArray(manifest.surfaces) ? manifest.surfaces : []),
    ...readSurfaceFragments(path.join(repoRoot, 'scripts/acceptance/coverage/surfaces')),
  ];
  const runtimeRoutes = runtimeRouteList(repoRoot);
  const routeBindings = discoverRouteBindings(repoRoot, runtimeRoutes);
  const discovered = discoverViews(repoRoot);

  return {
    contract: readJson(path.join(repoRoot, 'docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json')),
    manifestSurfaces,
    routeNameExclusions: Array.isArray(manifest.route_name_exclusions) ? manifest.route_name_exclusions : [],
    runtimeRoutes,
    routeBindings,
    views: discovered.views,
    structuralReferences: discovered.structuralReferences,
    navigationReferences: discovered.navigationReferences,
    repoRoot,
  };
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
