import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(coverageRoot, '../../..');
const productionRoots = ['app', 'resources/views', 'routes']
  .map((entry) => path.join(repoRoot, entry))
  .filter((entry) => fs.existsSync(entry));
const viewRoot = path.join(repoRoot, 'resources/views');
const defaultEvidencePath = path.join(repoRoot, 'docs/testing/PORTAL_ROUTE_VIEW_NAVIGATION_EVIDENCE.json');
const manifestPath = path.join(coverageRoot, 'portal-coverage-manifest.json');
const inspectorPath = path.join(coverageRoot, 'inspect-route-view-navigation.php');
const allowedViewClassifications = new Set(['framework_convention', 'dynamic_response', 'direct_entry', 'tracked_retirement']);
const allowedRouteKinds = new Set(['rendered_screen', 'action_endpoint', 'supporting_endpoint', 'redirect', 'justified_exception']);

function walk(root, predicate) {
  const files = [];
  const stack = [root];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const absolute = path.join(current, entry.name);
      if (entry.isDirectory()) stack.push(absolute);
      else if (entry.isFile() && predicate(absolute)) files.push(absolute);
    }
  }
  return files.sort((left, right) => left.localeCompare(right));
}

function relative(file) {
  return path.relative(repoRoot, file).split(path.sep).join('/');
}

function lineNumber(content, offset) {
  return content.slice(0, offset).split('\n').length;
}

function addLiteralMatches(target, file, content, regex, valueIndex, kind = 'literal') {
  for (const match of content.matchAll(regex)) {
    target.push({ value: match[valueIndex], file: relative(file), line: lineNumber(content, match.index ?? 0), marker: match[0], kind });
  }
}

function viewPath(viewName) {
  return path.join(viewRoot, `${viewName.replaceAll('.', '/')}.blade.php`);
}

function isPageLikeView(file) {
  const rel = relative(file);
  const basename = path.basename(file);
  const excludedSegments = ['/components/', '/emails/', '/layouts/', '/mail/', '/partials/', '/vendor/'];
  if (excludedSegments.some((segment) => `/${rel}`.includes(segment))) return false;
  if (basename === 'form.blade.php' || basename === 'layout.blade.php') return false;
  if (basename.startsWith('_')) return false;
  return true;
}

function isGlobalNavigationSource(file) {
  return /(?:^|\/)(?:components|layouts|partials)(?:\/|$)/u.test(file)
    || /(?:navigation|navbar|sidebar|menu)/iu.test(path.basename(file));
}

function evidencePathFromArguments() {
  const argument = process.argv.slice(2).find((entry) => entry.startsWith('--evidence='));
  if (!argument) return defaultEvidencePath;
  return path.resolve(process.cwd(), argument.slice('--evidence='.length).trim());
}

function readJson(file, label, errors) {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (error) {
    errors.push(`Cannot load ${label} ${relative(file)}: ${error.message}`);
    return null;
  }
}

function validateBoundedRecord(record, marker, key, allowedValues, errors) {
  const name = typeof record?.[key] === 'string' ? record[key].trim() : '';
  const rationale = typeof record?.rationale === 'string' ? record.rationale.trim() : '';
  const evidence = Array.isArray(record?.evidence) ? record.evidence.filter((entry) => typeof entry === 'string' && entry.trim() !== '') : [];
  if (name === '') errors.push(`Missing ${key} at ${marker}`);
  if (allowedValues && !allowedValues.has(name)) errors.push(`Unsupported ${key} ${name || '<empty>'} at ${marker}`);
  if (rationale.length < 20) errors.push(`Weak rationale at ${marker}`);
  if (evidence.length === 0) errors.push(`Missing evidence at ${marker}`);
  return { name, rationale, evidence };
}

function loadContract(file) {
  const errors = [];
  const document = readJson(file, 'route/view/navigation evidence', errors);
  if (!document) return { document: null, viewRecords: [], routeOverrides: [], directEntries: [], errors };
  if (document.schema_version !== 2) errors.push(`Unsupported route/view/navigation evidence schema at ${relative(file)}; expected schema_version 2`);

  const viewRecords = [];
  const seenViewPaths = new Set();
  if (!Array.isArray(document.unreferenced_page_classifications)) {
    errors.push(`Route/view/navigation evidence at ${relative(file)} must define unreferenced_page_classifications`);
  } else {
    for (const [index, record] of document.unreferenced_page_classifications.entries()) {
      const marker = `${relative(file)}:unreferenced_page_classifications[${index}]`;
      const recordErrors = [];
      const recordPath = typeof record?.path === 'string' ? record.path.trim() : '';
      const bounded = validateBoundedRecord(record, marker, 'classification', allowedViewClassifications, recordErrors);
      if (recordPath === '' || path.isAbsolute(recordPath) || recordPath.includes('\\') || recordPath.split('/').includes('..')) recordErrors.push(`Invalid view classification path at ${marker}`);
      if (recordPath !== '' && seenViewPaths.has(recordPath)) recordErrors.push(`Duplicate view classification ${recordPath}`);
      if (bounded.name === 'tracked_retirement' && (!Number.isInteger(record?.tracking_issue) || record.tracking_issue <= 0)) recordErrors.push(`Tracked retirement ${recordPath || marker} must reference a positive tracking_issue`);
      if (recordPath !== '') seenViewPaths.add(recordPath);
      errors.push(...recordErrors);
      if (recordErrors.length === 0) viewRecords.push({ path: recordPath, classification: bounded.name, rationale: bounded.rationale, evidence: bounded.evidence, ...(bounded.name === 'tracked_retirement' ? { tracking_issue: record.tracking_issue } : {}) });
    }
  }

  const routeOverrides = [];
  const seenOverrideRoutes = new Set();
  if (!Array.isArray(document.route_kind_overrides)) errors.push(`Route/view/navigation evidence at ${relative(file)} must define route_kind_overrides`);
  else {
    for (const [index, record] of document.route_kind_overrides.entries()) {
      const marker = `${relative(file)}:route_kind_overrides[${index}]`;
      const recordErrors = [];
      const route = typeof record?.route === 'string' ? record.route.trim() : '';
      const bounded = validateBoundedRecord(record, marker, 'kind', allowedRouteKinds, recordErrors);
      const views = Array.isArray(record?.views) ? record.views.filter((entry) => typeof entry === 'string' && entry.trim() !== '') : [];
      if (route === '') recordErrors.push(`Missing route at ${marker}`);
      if (route !== '' && seenOverrideRoutes.has(route)) recordErrors.push(`Duplicate route kind override ${route}`);
      if (bounded.name === 'rendered_screen' && views.length === 0) recordErrors.push(`Rendered route override ${route || marker} must define views`);
      if (bounded.name !== 'rendered_screen' && views.length > 0) recordErrors.push(`Non-rendered route override ${route || marker} must not define views`);
      if (route !== '') seenOverrideRoutes.add(route);
      errors.push(...recordErrors);
      if (recordErrors.length === 0) routeOverrides.push({ route, kind: bounded.name, rationale: bounded.rationale, evidence: bounded.evidence, views });
    }
  }

  const directEntries = [];
  const seenDirectRoutes = new Set();
  if (!Array.isArray(document.direct_entry_routes)) errors.push(`Route/view/navigation evidence at ${relative(file)} must define direct_entry_routes`);
  else {
    for (const [index, record] of document.direct_entry_routes.entries()) {
      const marker = `${relative(file)}:direct_entry_routes[${index}]`;
      const recordErrors = [];
      const route = typeof record?.route === 'string' ? record.route.trim() : '';
      const rationale = typeof record?.rationale === 'string' ? record.rationale.trim() : '';
      const evidence = Array.isArray(record?.evidence) ? record.evidence.filter((entry) => typeof entry === 'string' && entry.trim() !== '') : [];
      if (route === '') recordErrors.push(`Missing direct-entry route at ${marker}`);
      if (route !== '' && seenDirectRoutes.has(route)) recordErrors.push(`Duplicate direct-entry route ${route}`);
      if (rationale.length < 20) recordErrors.push(`Weak direct-entry rationale for ${route || marker}`);
      if (evidence.length === 0) recordErrors.push(`Missing direct-entry evidence for ${route || marker}`);
      if (route !== '') seenDirectRoutes.add(route);
      errors.push(...recordErrors);
      if (recordErrors.length === 0) directEntries.push({ route, rationale, evidence });
    }
  }
  return { document, viewRecords, routeOverrides, directEntries, errors };
}

const errors = [];
let inspection;
try {
  inspection = JSON.parse(execFileSync('php', [inspectorPath], { cwd: repoRoot, env: process.env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }));
} catch (error) {
  process.stderr.write(`Cannot inspect Laravel routes: ${error.stderr?.toString().trim() || error.message}\n`);
  process.exit(1);
}
if (inspection?.schema_version !== 1 || !Array.isArray(inspection?.routes)) {
  process.stderr.write('Invalid route inspection payload\n');
  process.exit(1);
}

const namedRoutes = new Map(inspection.routes.map((route) => [route.name, route]));
const manifest = readJson(manifestPath, 'portal coverage manifest', errors);
const routeOwners = new Map();
const supportingRoutes = new Set();
for (const surface of manifest?.surfaces ?? []) {
  for (const routeName of surface.route_names ?? []) {
    const owners = routeOwners.get(routeName) ?? [];
    owners.push({ id: surface.id, status: surface.status, owner: surface.owner });
    routeOwners.set(routeName, owners);
    if (surface.status === 'supporting_endpoint') supportingRoutes.add(routeName);
  }
}
const exclusions = manifest?.route_name_exclusions ?? [];
function exclusionFor(name) {
  return exclusions.find((entry) => entry.exact === name || (typeof entry.prefix === 'string' && name.startsWith(entry.prefix))) ?? null;
}

const sourceFiles = productionRoots.flatMap((root) => walk(root, (file) => file.endsWith('.php') || file.endsWith('.blade.php')));
const routeReferences = [];
const viewReferences = [];
const navigationReferences = [];
for (const file of sourceFiles) {
  const content = fs.readFileSync(file, 'utf8');
  addLiteralMatches(routeReferences, file, content, /(?<!->)(?<!::)\broute\(\s*(['"])([^'"]+)\1/gu, 2);
  addLiteralMatches(viewReferences, file, content, /(?<!->)(?<!::)\bview\(\s*(['"])([^'"]+)\1/gu, 2);
  addLiteralMatches(viewReferences, file, content, /->view\(\s*(['"])([^'"]+)\1/gu, 2);
  addLiteralMatches(viewReferences, file, content, /\bView::make\(\s*(['"])([^'"]+)\1/gu, 2);
  addLiteralMatches(viewReferences, file, content, /\bPassport::authorizationView\(\s*(['"])([^'"]+)\1/gu, 2);
  addLiteralMatches(viewReferences, file, content, /Route::view\(\s*[^,]+,\s*(['"])([^'"]+)\1/gu, 2);
  addLiteralMatches(viewReferences, file, content, /@(?:extends|include|includeIf|includeWhen|includeUnless|component)\(\s*(['"])([^'"]+)\1/gu, 2);
  addLiteralMatches(viewReferences, file, content, /@each\(\s*(['"])([^'"]+)\1/gu, 2);
  if (file.endsWith('.blade.php')) {
    addLiteralMatches(navigationReferences, file, content, /(?<!->)(?<!::)\broute\(\s*(['"])([^'"]+)\1/gu, 2, 'route_helper');
    addLiteralMatches(navigationReferences, file, content, /['"](?:route|route_name)['"]\s*=>\s*(['"])([^'"]+)\1/gu, 2, 'route_declaration');
  }
}

const brokenRouteReferences = routeReferences.filter((reference) => !namedRoutes.has(reference.value));
const brokenNavigationReferences = navigationReferences.filter((reference) => !namedRoutes.has(reference.value));
const brokenViewReferences = viewReferences.filter((reference) => !fs.existsSync(viewPath(reference.value)));
const referencedViews = new Set(viewReferences.map((reference) => reference.value));
const bladeFiles = walk(viewRoot, (file) => file.endsWith('.blade.php'));
const pageLikeViews = bladeFiles.filter(isPageLikeView);
const unreferencedPageCandidates = pageLikeViews.filter((file) => {
  const viewName = relative(file).replace(/^resources\/views\//u, '').replace(/\.blade\.php$/u, '').replaceAll('/', '.');
  return !referencedViews.has(viewName);
}).map(relative);

const contractFile = evidencePathFromArguments();
const contract = loadContract(contractFile);
const candidateSet = new Set(unreferencedPageCandidates);
const classificationByPath = new Map(contract.viewRecords.map((record) => [record.path, record]));
const unclassifiedPageCandidates = unreferencedPageCandidates.filter((candidate) => !classificationByPath.has(candidate));
const staleViewClassifications = contract.viewRecords.filter((record) => !candidateSet.has(record.path));
const classifiedUnreferencedPages = unreferencedPageCandidates.filter((candidate) => classificationByPath.has(candidate)).map((candidate) => classificationByPath.get(candidate));

const overrideByRoute = new Map(contract.routeOverrides.map((record) => [record.route, record]));
const directByRoute = new Map(contract.directEntries.map((record) => [record.route, record]));
const navigationByRoute = new Map();
for (const reference of navigationReferences) {
  const current = navigationByRoute.get(reference.value) ?? [];
  current.push({ file: reference.file, line: reference.line, kind: reference.kind, scope: isGlobalNavigationSource(reference.file) ? 'global' : 'contextual' });
  navigationByRoute.set(reference.value, current);
}

const staleRouteOverrides = contract.routeOverrides.filter((record) => !namedRoutes.has(record.route));
const staleDirectEntries = contract.directEntries.filter((record) => !namedRoutes.has(record.route));
const routeClassifications = [];
const unclassifiedRoutes = [];
const renderedRouteErrors = [];
const reachabilityErrors = [];

for (const [name, route] of [...namedRoutes.entries()].sort(([left], [right]) => left.localeCompare(right))) {
  const override = overrideByRoute.get(name);
  const owners = routeOwners.get(name) ?? [];
  const exclusion = exclusionFor(name);
  let kind = override?.kind ?? null;
  let rationale = override?.rationale ?? null;
  let classificationSource = override ? 'evidence_override' : 'source';

  if (!kind && exclusion) {
    kind = name.startsWith('storage.local') ? 'supporting_endpoint' : 'justified_exception';
    rationale = exclusion.reason;
    classificationSource = 'portal_manifest_exclusion';
  }
  if (!kind && supportingRoutes.has(name)) {
    kind = 'supporting_endpoint';
    rationale = `Portal surface ${owners.map((owner) => owner.id).join(', ')} owns this supporting resource endpoint.`;
    classificationSource = 'portal_manifest_surface';
  }
  const methods = Array.isArray(route.methods) ? route.methods : [];
  const isReadRoute = methods.length > 0 && methods.every((method) => method === 'GET' || method === 'HEAD');
  if (!kind && !isReadRoute) {
    kind = 'action_endpoint';
    rationale = 'The named route accepts a state-changing or form-submission HTTP method and is not a standalone rendered screen.';
  }
  if (!kind && (String(route.action).includes('RedirectController') || route.defaults?.destination)) {
    kind = 'redirect';
    rationale = 'Laravel route metadata identifies a redirect controller or redirect destination.';
  }
  const sourceViews = [...new Set([...(route.views ?? []).map((entry) => entry.name), ...(override?.views ?? [])])];
  if (!kind && sourceViews.length > 0) {
    kind = 'rendered_screen';
    rationale = 'Reflected route implementation resolves one or more explicit Blade views.';
  }
  if (!kind && route.has_redirect && !route.has_resource_response && !route.has_dynamic_view) {
    kind = 'redirect';
    rationale = 'The reflected route implementation redirects and contains no rendered-view or resource-response path.';
  }
  if (!kind) {
    unclassifiedRoutes.push({ name, methods, uri: route.uri, action: route.action, owners, views: route.views, sources: route.sources, hints: { has_redirect: route.has_redirect, has_resource_response: route.has_resource_response, has_dynamic_view: route.has_dynamic_view } });
    continue;
  }

  const navigation = navigationByRoute.get(name) ?? [];
  let reachability = null;
  if (kind === 'rendered_screen') {
    if (sourceViews.length === 0) renderedRouteErrors.push(`Rendered route ${name} has no exact Blade view binding`);
    for (const view of sourceViews) if (!fs.existsSync(viewPath(view))) renderedRouteErrors.push(`Rendered route ${name} binds missing Blade view ${view}`);
    if (navigation.length > 0) {
      reachability = navigation.some((entry) => entry.scope === 'global') ? 'global_navigation' : 'contextual_navigation';
    } else if (directByRoute.has(name)) {
      reachability = 'direct_entry';
    } else {
      reachabilityErrors.push(`Rendered route ${name} has no global/contextual navigation reference or bounded direct-entry rationale`);
    }
  }

  routeClassifications.push({ name, kind, methods, uri: route.uri, action: route.action, owners, classification_source: classificationSource, rationale, views: sourceViews, source_markers: route.sources ?? [], navigation, reachability, direct_entry: directByRoute.get(name) ?? null });
}

const invalidDirectEntries = contract.directEntries.filter((record) => {
  const classified = routeClassifications.find((entry) => entry.name === record.route);
  return classified && (classified.kind !== 'rendered_screen' || classified.navigation.length > 0);
});
const duplicateRouteOwners = [...routeOwners.entries()].filter(([, owners]) => owners.length > 1).map(([route, owners]) => ({ route, owners }));
const unownedRoutes = [...namedRoutes.keys()].filter((name) => (routeOwners.get(name)?.length ?? 0) === 0 && !exclusionFor(name));

const reportErrors = [
  ...errors,
  ...brokenRouteReferences.map((reference) => `Unknown named route ${reference.value} at ${reference.file}:${reference.line}`),
  ...brokenNavigationReferences.map((reference) => `Broken navigation route ${reference.value} at ${reference.file}:${reference.line}`),
  ...brokenViewReferences.map((reference) => `Missing Blade view ${reference.value} at ${reference.file}:${reference.line}`),
  ...contract.errors,
  ...unclassifiedPageCandidates.map((candidate) => `Unclassified page-like Blade view ${candidate}`),
  ...staleViewClassifications.map((record) => `Stale view classification ${record.path}: path is no longer an unreferenced page-like Blade candidate`),
  ...staleRouteOverrides.map((record) => `Stale route kind override ${record.route}`),
  ...staleDirectEntries.map((record) => `Stale direct-entry route ${record.route}`),
  ...invalidDirectEntries.map((record) => `Stale direct-entry rationale ${record.route}: route is not an unlinked rendered screen`),
  ...duplicateRouteOwners.map((entry) => `Duplicate portal surface ownership for ${entry.route}: ${entry.owners.map((owner) => owner.id).join(', ')}`),
  ...unownedRoutes.map((name) => `Named route ${name} has no portal surface ownership or bounded manifest exclusion`),
  ...unclassifiedRoutes.map((route) => `Unclassified named route ${route.name}`),
  ...renderedRouteErrors,
  ...reachabilityErrors,
];

const kindCounts = Object.fromEntries([...allowedRouteKinds].map((kind) => [kind, routeClassifications.filter((route) => route.kind === kind).length]));
const report = {
  schema_version: 3,
  classification_file: relative(contractFile),
  named_route_count: namedRoutes.size,
  classified_route_count: routeClassifications.length,
  route_kind_counts: kindCounts,
  literal_route_reference_count: routeReferences.length,
  navigation_reference_count: navigationReferences.length,
  literal_view_reference_count: viewReferences.length,
  blade_view_count: bladeFiles.length,
  page_like_view_count: pageLikeViews.length,
  broken_route_references: brokenRouteReferences,
  broken_navigation_references: brokenNavigationReferences,
  broken_view_references: brokenViewReferences,
  unreferenced_page_candidates: unreferencedPageCandidates,
  classified_unreferenced_pages: classifiedUnreferencedPages,
  unclassified_page_candidates: unclassifiedPageCandidates,
  stale_view_classifications: staleViewClassifications,
  duplicate_route_owners: duplicateRouteOwners,
  unowned_routes: unownedRoutes,
  unclassified_routes: unclassifiedRoutes,
  route_classifications: routeClassifications,
  errors: reportErrors,
};
process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
if (report.errors.length > 0) process.exitCode = 1;
