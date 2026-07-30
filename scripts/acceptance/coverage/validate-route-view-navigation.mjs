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
const allowedViewClassifications = new Set([
  'framework_convention',
  'dynamic_response',
  'direct_entry',
  'tracked_retirement',
]);

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

function evidencePathFromArguments() {
  const argument = process.argv.slice(2).find((entry) => entry.startsWith('--evidence='));
  if (!argument) return defaultEvidencePath;
  const requested = argument.slice('--evidence='.length).trim();
  return path.resolve(process.cwd(), requested);
}

function loadViewClassifications(file) {
  const errors = [];
  let document;

  try {
    document = JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (error) {
    return {
      records: [],
      errors: [`Cannot load route/view/navigation evidence ${relative(file)}: ${error.message}`],
    };
  }

  if (document?.schema_version !== 1) {
    errors.push(`Unsupported route/view/navigation evidence schema at ${relative(file)}; expected schema_version 1`);
  }

  if (!Array.isArray(document?.unreferenced_page_classifications)) {
    errors.push(`Route/view/navigation evidence at ${relative(file)} must define unreferenced_page_classifications`);
    return { records: [], errors };
  }

  const records = [];
  const seenPaths = new Set();

  for (const [index, record] of document.unreferenced_page_classifications.entries()) {
    const marker = `${relative(file)}:unreferenced_page_classifications[${index}]`;
    const recordErrors = [];
    const recordPath = typeof record?.path === 'string' ? record.path.trim() : '';
    const classification = typeof record?.classification === 'string' ? record.classification.trim() : '';
    const rationale = typeof record?.rationale === 'string' ? record.rationale.trim() : '';
    const evidence = Array.isArray(record?.evidence)
      ? record.evidence.filter((entry) => typeof entry === 'string' && entry.trim() !== '')
      : [];

    if (recordPath === '' || path.isAbsolute(recordPath) || recordPath.includes('\\') || recordPath.split('/').includes('..')) {
      recordErrors.push(`Invalid view classification path at ${marker}`);
    }
    if (recordPath !== '' && seenPaths.has(recordPath)) {
      recordErrors.push(`Duplicate view classification ${recordPath}`);
    }
    if (!allowedViewClassifications.has(classification)) {
      recordErrors.push(`Unsupported view classification ${classification || '<empty>'} for ${recordPath || marker}`);
    }
    if (rationale.length < 20) {
      recordErrors.push(`Weak view classification rationale for ${recordPath || marker}`);
    }
    if (evidence.length === 0) {
      recordErrors.push(`Missing view classification evidence for ${recordPath || marker}`);
    }
    if (classification === 'tracked_retirement' && (!Number.isInteger(record?.tracking_issue) || record.tracking_issue <= 0)) {
      recordErrors.push(`Tracked retirement ${recordPath || marker} must reference a positive tracking_issue`);
    }

    if (recordPath !== '') seenPaths.add(recordPath);
    errors.push(...recordErrors);
    if (recordErrors.length > 0) continue;

    records.push({
      path: recordPath,
      classification,
      rationale,
      evidence,
      ...(classification === 'tracked_retirement' ? { tracking_issue: record.tracking_issue } : {}),
    });
  }

  return { records, errors };
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

const classificationFile = evidencePathFromArguments();
const classificationContract = loadViewClassifications(classificationFile);
const candidateSet = new Set(unreferencedPageCandidates);
const classificationByPath = new Map(classificationContract.records.map((record) => [record.path, record]));
const unclassifiedPageCandidates = unreferencedPageCandidates
  .filter((candidate) => !classificationByPath.has(candidate));
const staleViewClassifications = classificationContract.records
  .filter((record) => !candidateSet.has(record.path));
const classifiedUnreferencedPages = unreferencedPageCandidates
  .filter((candidate) => classificationByPath.has(candidate))
  .map((candidate) => classificationByPath.get(candidate));

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
  schema_version: 2,
  classification_file: relative(classificationFile),
  named_route_count: namedRoutes.size,
  get_like_route_count: getLikeRoutes.length,
  literal_route_reference_count: routeReferences.length,
  literal_view_reference_count: viewReferences.length,
  blade_view_count: bladeFiles.length,
  page_like_view_count: pageLikeViews.length,
  broken_route_references: brokenRouteReferences,
  broken_view_references: brokenViewReferences,
  unreferenced_page_candidates: unreferencedPageCandidates,
  classified_unreferenced_pages: classifiedUnreferencedPages,
  unclassified_page_candidates: unclassifiedPageCandidates,
  stale_view_classifications: staleViewClassifications,
  get_like_routes: getLikeRoutes,
  errors: [
    ...brokenRouteReferences.map((reference) => `Unknown named route ${reference.value} at ${reference.file}:${reference.line}`),
    ...brokenViewReferences.map((reference) => `Missing Blade view ${reference.value} at ${reference.file}:${reference.line}`),
    ...classificationContract.errors,
    ...unclassifiedPageCandidates.map((candidate) => `Unclassified page-like Blade view ${candidate}`),
    ...staleViewClassifications.map((record) => `Stale view classification ${record.path}: path is no longer an unreferenced page-like Blade candidate`),
  ],
};

process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
if (report.errors.length > 0) process.exitCode = 1;
