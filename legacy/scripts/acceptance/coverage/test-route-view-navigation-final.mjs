import assert from 'node:assert/strict';
import {
  loadRepositoryInputs,
  validateRouteViewNavigationInventory,
} from './validate-route-view-navigation-final.mjs';

function cloneInputs() {
  const input = loadRepositoryInputs();
  return {
    ...input,
    contract: structuredClone(input.contract),
    manifestSurfaces: structuredClone(input.manifestSurfaces),
    routeNameExclusions: structuredClone(input.routeNameExclusions),
    runtimeRoutes: structuredClone(input.runtimeRoutes),
    routeBindings: structuredClone(input.routeBindings),
    views: structuredClone(input.views),
    structuralReferences: structuredClone(input.structuralReferences),
    navigationReferences: structuredClone(input.navigationReferences),
  };
}

function expectFailure(label, mutate, marker) {
  const input = cloneInputs();
  mutate(input);
  const report = validateRouteViewNavigationInventory(input);
  assert.ok(report.errors.some((error) => error.includes(marker)), `${label}: ${JSON.stringify(report, null, 2)}`);
}

const input = loadRepositoryInputs();
const baseline = validateRouteViewNavigationInventory(input);
assert.deepEqual(baseline.errors, [], JSON.stringify(baseline, null, 2));
assert.equal(baseline.runtime_route_count, baseline.classified_route_count);
assert.equal(baseline.orphan_view_count, 0);
assert.equal(baseline.strict_closure, true);

const rendered = baseline.inventory.find((record) => record.kind === 'rendered_screen' && record.views.length > 0);
assert.ok(rendered);
const navigated = baseline.inventory.find((record) => record.kind === 'rendered_screen'
  && input.navigationReferences.some((reference) => reference.route_name === record.route_name)
  && !(input.contract.direct_entry_routes ?? []).some((entry) => entry.exact === record.route_name));
assert.ok(navigated);

expectFailure('schema', ({ contract }) => { contract.schema_version = 2; }, 'schema_version must be 1');
expectFailure('unowned route', ({ runtimeRoutes }) => {
  runtimeRoutes.push({ method: 'GET|HEAD', uri: 'fixture/unowned', name: 'fixture.unowned', action: 'Closure' });
}, 'not owned by the portal manifest');
expectFailure('duplicate owner', ({ manifestSurfaces }) => {
  const owner = manifestSurfaces.find((surface) => surface.route_names?.includes(rendered.route_name));
  manifestSurfaces.find((surface) => surface !== owner).route_names.push(rendered.route_name);
}, 'is owned by both');
expectFailure('missing view', ({ views }) => { views.delete(rendered.views[0].name); }, 'references missing Blade view');
expectFailure('missing marker', ({ routeBindings }) => {
  routeBindings.get(rendered.route_name).views[0].source.markers = ['fixture-missing-marker'];
}, 'source marker not found');
expectFailure('orphan view', ({ views }) => {
  views.set('fixture.orphan', { name: 'fixture.orphan', file: 'resources/views/fixture/orphan.blade.php', content: '<main />' });
}, 'Page-like Blade view is not bound');
expectFailure('broken navigation', ({ navigationReferences }) => {
  navigationReferences.push({ route_name: 'fixture.missing', kind: 'contextual', source: structuredClone(navigationReferences[0].source) });
}, 'Navigation source references unknown named route');
expectFailure('missing reachability', ({ navigationReferences, contract }) => {
  navigationReferences.splice(0, navigationReferences.length, ...navigationReferences.filter((entry) => entry.route_name !== navigated.route_name));
  contract.direct_entry_routes = contract.direct_entry_routes.filter((entry) => entry.exact !== navigated.route_name);
}, 'has no global/contextual navigation reference');
expectFailure('weak direct entry', ({ contract }) => { contract.direct_entry_routes[0].rationale = 'weak'; }, 'requires a bounded rationale');
expectFailure('stale override', ({ contract }) => {
  contract.route_overrides.push({ exact: 'fixture.absent', kind: 'exception', rationale: 'This exact fixture override is deliberately long enough but matches no runtime route and therefore must fail closed.' });
}, 'route override matches no exact runtime route');
expectFailure('unclassified read route', ({ runtimeRoutes, manifestSurfaces, routeBindings }) => {
  runtimeRoutes.push({ method: 'GET|HEAD', uri: 'fixture/unclassified', name: 'fixture.unclassified', action: 'Closure' });
  manifestSurfaces[0].route_names.push('fixture.unclassified');
  routeBindings.set('fixture.unclassified', { views: [], redirect: null });
}, 'GET/HEAD route cannot be classified');
expectFailure('weak exclusion', ({ contract }) => {
  contract.view_exclusions.push({ exact: 'fixture.view', rationale: 'weak' });
}, 'requires a bounded rationale');

process.stdout.write(`${JSON.stringify({
  portal_surfaces: baseline.portal_surface_count,
  classified_routes: baseline.classified_route_count,
  rendered_routes: baseline.rendered_route_count,
  bound_views: baseline.bound_view_count,
  navigation_references: baseline.navigation_reference_count,
  direct_entry_routes: baseline.direct_entry_route_count,
  orphan_views: baseline.orphan_view_count,
  negative_fixtures: 12,
  strict_closure: baseline.strict_closure,
  status: 'pass',
}, null, 2)}\n`);
