# Route, View and Navigation Closure Evidence — 2026-07-30

## Scope

This evidence closes the bounded repository inventory requested by Issue #360 under parent Issue #326. It classifies delivered named Laravel routes, binds route responses to Blade views and exact source markers, validates static global/contextual navigation references, and requires bounded direct-entry or exclusion records where static navigation is intentionally absent.

It does not declare the portal `PRODUCT_COMPLETE` and does not close unrelated state, browser, accessibility, visual or production-verification gaps tracked by Issue #326.

## Authoritative validation

```text
exact head: 0eb6780a79e4aecfe66ce54642f1541a87f0f31b
workflow: Portal Acceptance Contract
run: 30583555606
job: Strict portal coverage closure
job id: 91009499982
result: PASS
retries: 0 for deterministic contract execution
artifact: portal-coverage-report-30583555606-1
artifact digest: sha256:c6a3b52535410927b64bd6235b6961bceb98c3d92df38e46d12579e4ba558bef
```

Command executed by the strict workflow:

```text
npm --prefix scripts/acceptance run test:coverage-contract:strict
```

## Inventory result

```json
{
  "portal_surfaces": 27,
  "runtime_named_routes": 228,
  "classified_routes": 228,
  "route_kinds": {
    "rendered_screen": 126,
    "form_action": 76,
    "redirect": 16,
    "resource_supporting": 10,
    "exception": 0
  },
  "blade_views": 121,
  "bound_views": 95,
  "structural_views": 26,
  "bounded_excluded_views": 2,
  "orphan_views": 0,
  "navigation_references": 400,
  "bounded_direct_entry_routes": 30,
  "errors": 0,
  "warnings": 0,
  "strict_closure": true
}
```

## Deterministic negative evidence

The fixture suite passed 12 fail-closed mutations covering:

- invalid schema version;
- an unowned runtime route;
- duplicate route ownership;
- a missing bound Blade view;
- a missing exact implementation marker;
- an orphan page-like view;
- a navigation reference to an unknown route;
- a rendered route without navigation or direct-entry evidence;
- a weak direct-entry rationale;
- a stale route override;
- an unclassifiable readable route;
- a weak view-exclusion rationale.

## Bounded policies

- Locale-neutral `legacy.*` compatibility entries are enumerated individually as direct-entry routes; canonical navigation uses localized route names.
- Token/signed-flow pages are enumerated individually because ordinary navigation must not expose them without the required continuation state.
- `seo.robots`, `seo.sitemap` and the administrator Wiki media JSON library are classified as supporting resources, not interactive screens.
- `game-auth.oauth.authorize` is excluded from portal-page ownership because its framework route family is governed by Identity/security contracts.
- `home-preview` is retained as an explicit noindex design-reference exclusion pending separate retirement; it is not represented as a delivered route.
- `support.editorial.show` is bound to `support.index` and `support.report-a-bug` through the exact delegated renderer `App\Support\PublicEditorialPage::render` rather than hidden by an exclusion.

## Nonclaims

- Repository closure does not prove deployment or production reachability.
- Static route/view/navigation closure does not prove every runtime data state, role, viewport, browser or assistive-technology permutation.
- No production, staging, Canary, schema, payment-provider or user-data mutation was performed.
- Parent Issue #326 remains open for independent review and unrelated completeness gaps.
