import assert from 'node:assert/strict';
import {
  loadRepositoryInputs,
  validateRouteViewNavigationInventory,
} from './validate-route-view-navigation-inventory.mjs';

function cloneInputs() {
  const baseline = loadRepositoryInputs();
  return {
    contract: structuredClone(baseline.contract),
    manifestSurfaces: structuredClone(baseline.manifestSurfaces),
    routeNameExclusions: structuredClone(baseline.routeNameExclusions),
    runtimeRoutes: structuredClone(baseline.runtimeRoutes),
    routeBindings: structuredClone(baseline.routeBindings),
    views: structuredClone(baseline.views),
    structuralReferences: structuredClone(baseline.structuralReferences),
    navigationReferences: structuredClone(baseline.navigationReferences),
    repoRoot: baseline.repoRoot,
  };
}

function expectFailure(name, mutate, expectedMarker) {
  const inputs = cloneInputs();
  mutate(inputs);
  const report = validateRouteViewNavigationInventory(inputs);
  assert.ok(
    report.errors.some((error) => error.includes(expectedMarker)),
    `${name} did not fail with ${JSON.stringify(expectedMarker)}.\n${JSON.stringify(report, null, 2)}`,
  );
}

const baselineInputs = loadRepositoryInputs();
const baselineReport = validateRouteViewNavigationInventory(baselineInputs);
assert.deepEqual(baselineReport.errors, [], JSON.stringify(baselineReport, null, 2));
assert.equal(baselineReport.strict_closure, true, JSON.stringify(baselineReport, null, 2));
assert.equal(baselineReport.runtime_route_count, baselineReport.classified_route_count, JSON.stringify(baselineReport, null, 2));
assert.equal(baselineReport.orphan_view_count, 0, JSON.stringify(baselineReport, null, 2));

const renderedRecord = baselineReport.inventory.find((record) => record.kind === 'rendered_screen' && record.views.length > 0);
assert.ok(renderedRecord, 'Baseline must contain at least one rendered route with an exact Blade binding.');
const navigatedRenderedRecord = baselineReport.inventory.find((record) => (
  record.kind === 'rendered_screen'
  && baselineInputs.navigationReferences.some((reference) => reference.route_name === record.route_name)
  && !(baselineInputs.contract.direct_entry_routes ?? []).some((entry) => (
    (entry.exact && entry.exact === record.route_name)
    || (entry.prefix && record.route_name.startsWith(entry.prefix))
  ))
));
assert.ok(navigatedRenderedRecord, 'Baseline must contain at least one rendered route reached through navigation evidence.');

expectFailure('invalid schema version', ({ contract }) => {
  contract.schema_version = 2;
}, 'schema_version must be 1');

expectFailure('unowned runtime route', ({ runtimeRoutes }) => {
  runtimeRoutes.push({
    method: 'GET|HEAD',
    uri: 'fixture/unowned',
    name: 'fixture.unowned',
    action: 'Closure',
  });
}, 'Named Laravel route is not owned by the portal manifest');

expectFailure('duplicate portal route ownership', ({ manifestSurfaces }) => {
  const owner = manifestSurfaces.find((surface) => surface.route_names?.includes(renderedRecord.route_name));
  const second = manifestSurfaces.find((surface) => surface !== owner);
  second.route_names.push(renderedRecord.route_name);
}, 'is owned by both');

expectFailure('missing bound Blade view', ({ views }) => {
  views.delete(renderedRecord.views[0].name);
}, 'references missing Blade view');

expectFailure('missing exact implementation marker', ({ routeBindings }) => {
  const binding = routeBindings.get(renderedRecord.route_name);
  binding.views[0].source.markers = ['marker-that-does-not-exist'];
}, 'source marker not found');

expectFailure('orphan page-like Blade view', ({ views }) => {
  views.set('fixture.orphan-page', {
    name: 'fixture.orphan-page',
    file: 'resources/views/fixture/orphan-page.blade.php',
    content: '<main>orphan</main>',
  });
}, 'Page-like Blade view is not bound');

expectFailure('broken navigation route', ({ navigationReferences }) => {
  const source = structuredClone(navigationReferences[0].source);
  navigationReferences.push({
    route_name: 'fixture.missing-navigation-route',
    kind: 'contextual',
    source,
  });
}, 'Navigation source references unknown named route');

expectFailure('rendered route without reachability evidence', ({ navigationReferences, contract }) => {
  const routeName = navigatedRenderedRecord.route_name;
  const retained = navigationReferences.filter((reference) => reference.route_name !== routeName);
  navigationReferences.splice(0, navigationReferences.length, ...retained);
  contract.direct_entry_routes = (contract.direct_entry_routes ?? []).filter((entry) => (
    entry.exact !== routeName && !(entry.prefix && routeName.startsWith(entry.prefix))
  ));
}, 'Rendered route has no global/contextual navigation reference or direct-entry rationale');

expectFailure('weak direct-entry rationale', ({ contract }) => {
  contract.direct_entry_routes[0].rationale = 'direct';
}, 'requires a bounded rationale');

expectFailure('stale route override', ({ contract }) => {
  contract.route_overrides.push({
    exact: 'fixture.route-that-does-not-exist',
    kind: 'exception',
    rationale: 'This deliberately invalid exact override has enough explanatory text but must fail because no runtime route matches it.',
  });
}, 'route override matches no exact runtime route');

expectFailure('unclassified readable route', ({ runtimeRoutes, manifestSurfaces, routeBindings }) => {
  runtimeRoutes.push({
    method: 'GET|HEAD',
    uri: 'fixture/unclassified',
    name: 'fixture.unclassified',
    action: 'Closure',
  });
  manifestSurfaces[0].route_names.push('fixture.unclassified');
  routeBindings.set('fixture.unclassified', { views: [], redirect: null });
}, 'GET/HEAD route cannot be classified');

expectFailure('weak view exclusion', ({ contract }) => {
  contract.view_exclusions.push({ exact: 'fixture.view', rationale: 'weak' });
}, 'requires a bounded rationale');

process.stdout.write(`${JSON.stringify({
  portal_surfaces: baselineReport.portal_surface_count,
  classified_routes: baselineReport.classified_route_count,
  rendered_routes: baselineReport.rendered_route_count,
  bound_views: baselineReport.bound_view_count,
  navigation_references: baselineReport.navigation_reference_count,
  direct_entry_routes: baselineReport.direct_entry_route_count,
  orphan_views: baselineReport.orphan_view_count,
  negative_fixtures: 12,
  strict_closure: baselineReport.strict_closure,
  status: 'pass',
}, null, 2)}\n`);
