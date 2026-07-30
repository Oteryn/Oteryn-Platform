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

function walk(root, predicate) {
  const files = [];
  const stack = [root];

  while (stack.length > 0) {
    const current = stack.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const absolute = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(absolute);
      } else if (entry.isFile() && predicate(absolute)) {
        files.push(absolute);
      }
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

function addLiteralMatches(target, file, content, regex, valueIndex) {
  for (const match of content.matchAll(regex)) {
    target.push({
      value: match[valueIndex],
      file: relative(file),
      line: lineNumber(content, match.index ?? 0),
      marker: match[0],
    });
  }
}

function viewPath(viewName) {
  return path.join(viewRoot, `${viewName.replaceAll('.', '/')}.blade.php`);
}

function isPageLikeView(file) {
  const rel = relative(file);
  const basename = path.basename(file);
  const excludedSegments = [
    '/components/',
    '/emails/',
    '/layouts/',
    '/mail/',
    '/partials/',
    '/vendor/',
  ];

  if (excludedSegments.some((segment) => `/${rel}`.includes(segment))) return false;
  if (basename === 'form.blade.php' || basename === 'layout.blade.php') return false;
  if (basename.startsWith('_')) return false;
  return true;
}

let routeList;
try {
  routeList = JSON.parse(execFileSync('php', ['artisan', 'route:list', '--json'], {
    cwd: repoRoot,
    env: process.env,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }));
} catch (error) {
  process.stderr.write(`Cannot obtain Laravel route list: ${error.stderr?.toString().trim() || error.message}\n`);
  process.exit(1);
}

const namedRoutes = new Map();
for (const route of Array.isArray(routeList) ? routeList : []) {
  const name = typeof route?.name === 'string' ? route.name.trim() : '';
  if (name === '') continue;
  namedRoutes.set(name, {
    methods: Array.isArray(route.method) ? route.method : String(route.method ?? '').split('|').filter(Boolean),
    uri: route.uri ?? null,
    action: route.action ?? null,
    middleware: Array.isArray(route.middleware) ? route.middleware : [],
  });
}

const sourceFiles = productionRoots.flatMap((root) => walk(root, (file) => (
  file.endsWith('.php') || file.endsWith('.blade.php')
)));
const routeReferences = [];
const viewReferences = [];

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
}

const brokenRouteReferences = routeReferences.filter((reference) => !namedRoutes.has(reference.value));
const brokenViewReferences = viewReferences.filter((reference) => !fs.existsSync(viewPath(reference.value)));
const referencedViews = new Set(viewReferences.map((reference) => reference.value));
const bladeFiles = walk(viewRoot, (file) => file.endsWith('.blade.php'));
const pageLikeViews = bladeFiles.filter(isPageLikeView);
const unreferencedPageCandidates = pageLikeViews
  .filter((file) => {
    const viewName = relative(file)
      .replace(/^resources\/views\//u, '')
      .replace(/\.blade\.php$/u, '')
      .replaceAll('/', '.');
    return !referencedViews.has(viewName);
  })
  .map(relative);

const routeReferenceOwners = new Map();
for (const reference of routeReferences) {
  const owners = routeReferenceOwners.get(reference.value) ?? [];
  owners.push({ file: reference.file, line: reference.line });
  routeReferenceOwners.set(reference.value, owners);
}

const getLikeRoutes = [...namedRoutes.entries()]
  .filter(([, route]) => route.methods.some((method) => method === 'GET' || method === 'HEAD'))
  .map(([name, route]) => ({
    name,
    ...route,
    literal_reference_count: routeReferenceOwners.get(name)?.length ?? 0,
  }))
  .sort((left, right) => left.name.localeCompare(right.name));

const report = {
  schema_version: 1,
  named_route_count: namedRoutes.size,
  get_like_route_count: getLikeRoutes.length,
  literal_route_reference_count: routeReferences.length,
  literal_view_reference_count: viewReferences.length,
  blade_view_count: bladeFiles.length,
  page_like_view_count: pageLikeViews.length,
  broken_route_references: brokenRouteReferences,
  broken_view_references: brokenViewReferences,
  unreferenced_page_candidates: unreferencedPageCandidates,
  get_like_routes: getLikeRoutes,
  errors: [
    ...brokenRouteReferences.map((reference) => `Unknown named route ${reference.value} at ${reference.file}:${reference.line}`),
    ...brokenViewReferences.map((reference) => `Missing Blade view ${reference.value} at ${reference.file}:${reference.line}`),
  ],
};

process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
if (report.errors.length > 0) process.exitCode = 1;
