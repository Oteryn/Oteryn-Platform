# Shared-template portal qualification

Browser qualification: **PASS**. Full PostgreSQL workspace acceptance is running;
its result is not yet claimed.

Fetched base: `26c767eec36c04b5227a9676ec827ec76c4e82e9`.
The complete browser run started at source `7bcfe31a6b53ac55115ef642ae37fa23aa0cee0a`.
Subsequent commits contain evidence and task-record updates only. The tested source
digest matches `46978e4211aa6c76ece921cdc13cfcc0307bdb1d45c3e8cc1364faff277fdcb0`. Its exclusions are only COVERAGE.md,
REPORT.md and qualification.json. All original flows from the fetched suite remain.

## Executed browser results

| Project | Passed | Failed | Skipped | Retries |
| --- | --- | --- | --- | --- |
| pl-1440 | 437 | 0 | 0 | 0 |
| pl-390 | 437 | 0 | 0 | 0 |
| en-1440 | 437 | 0 | 0 | 0 |
| en-390 | 437 | 0 | 0 | 0 |
| Total | 1748 | 0 | 0 | 0 |

The single complete run finished in 3.06 hours, with zero runner
errors and one attempt for every case. It used installed `/usr/bin/chromium`;
no browsers were installed. Native Node was used because the environment's wrapper
does not forward the browser fixture environment.

## Coverage totals

- 318 route rows: **317 covered / 1 NOT_APPLICABLE / 0 uncovered**.
- All 48 registry sections covered in every locale/viewport project.
- The compatibility Bazaar item-action adapter is not mounted when the full Bazaar
  runtime is selected. Its exact POST 404 absence passed in all four projects;
  current bid/control endpoints and positive Bazaar flows passed separately.
- Missing fixture references assert exact 400/404 refusals. Cold Game projections
  assert documented 503/no-store responses. These are executed assertions, never
  skipped tests or fabricated Game success.

COVERAGE.md maps every row to its module, flags, actor, spec/case and executed
result. Full HTML cases check the shared header/nav/footer, framed content, one
h1, current navigation, local links, visible keyboard focus, skip target, menus,
overflow and serious/critical axe violations. APIs, fragments, signed webhooks
and resources receive their protocol checks. Rendered form mutations require
missing-token refusal; signed one-click mail uses its documented 422 invalid-token
response, while session-protected forms require 403.

## Defects found and repaired

| Root cause | Fix and owning regression |
| --- | --- |
| Phone CSS hides dropdowns even on focus; anchor triggers cannot reliably toggle on touch | Native details/summary disclosure, preserved focus-within/hover, bounded menu scrolling; UI disclosure regression plus real keyboard/touch activation |
| Desktop last-child alignment clips the Account dropdown in the phone grid | Reset odd phone columns to left alignment; UI CSS regression and keyboard/touch bounds checks for every group |
| Parent and child sections both claim the current page | Select only the most specific current section; UI current-section regression |
| Unprefixed aliases never match localized registry URLs | Compare canonical route suffixes; UI alias/current-section regression |
| Standalone pages and the injected shell each render a skip link | Adapter retains one shared skip link; Web composition regression |
| Standalone main landmarks lack a focus target or use a custom ID | Add focusability while preserving custom IDs and provide a shared panel target; Web landmark regressions plus actual keyboard activation |
| Language links invent localized aliases or unsupported language query parameters | Preserve explicit localized routes; use the existing language endpoint for unprefixed routes; UI route-shape regressions |
| Standalone CSPs omit local image/font sources needed by the shared shell | Adapter adds only absent same-origin asset directives and preserves explicit restrictions; Web policy regression |
| Account shortcuts and sudo completion target an unmounted security landing route | Restore the localized landing alias to the existing credentials page; owning binary language/redirect regression |
| Public party board exposes a staff-only queue link | Restrict the link to the moderation surface; Community template regression |
| Wiki revision source blocks scroll without keyboard access | Focusable pre elements; Wiki rendered-template regression and positive publication/history browser flow |
| OAuth consent no-referrer policy turns the native form Origin opaque | HTML uses strict-origin, keeping queries out of referrers and preserving the real POST origin; protocol JSON retains no-referrer; Gameauth policy regression and browser consent/PKCE flow |
| Journal mutations parse forms before checking CSRF and bypass the shared origin guard | Standard session/CSRF middleware precedes extraction, rendered fields use _csrf; PostgreSQL owner, missing-token and foreign-origin regression |
| Staff control refusal documents omit a title | Complete localized document title and h1; Admin response regression |

## Harness corrections and regressions

- The continuing closure producer and local ownership readers previously trusted
  different source authorities. They now share one setting; privacy asserts
  agreement with the live feed before seeding isolated read models. Production
  authority checks remain intact. Privacy/world-read passed 12 targeted cases.
- World statistics has a healthy 200 registry selector. Privacy now asserts no
  character disclosure on that page and submits a registered world/date query,
  retaining exact 503/no-store data refusal. Healthy missing-character aliases
  require exact 404, confirmed by eight targeted cases across all projects.
- Existing shell selectors and logout handling now match the shared template.
  Relative form actions resolve against the current page. Token refusal strips
  both accepted CSRF aliases, and removes the signed mail authorization token for
  the separate one-click protocol check. OAuth empty-client refusal remains 401;
  a known local client with an invalid grant requires 400.
- Multipart fixtures contain a valid PNG with correct checksums. Staff browser
  state is stored only in private scratch with mode 0600. The summarizer rejects
  runner errors, incomplete projects, skips, retries and uncovered rows.
- Three existing optional Rust browser cases were explicitly executed using native
  Node and installed Chromium: all three passed, with zero axe violations. They
  will be executed again as part of final acceptance at the qualified source.

## Environment and limits

The launcher created fresh disposable databases, migrated, and ran serve plus
worker with APP_ENV=testing and PORTAL_PUBLIC_PROJECTIONS_ENABLED=true. All 87
literal environment switches ending ENABLED were enabled, including source-required
value 1 for the two delivery/world switches; explicit non-suffix module switches,
analytics ingestion/presentation and one local OAuth client were enabled too.
No environment switch had to be disabled. ENVIRONMENT.md lists the switches and
local producers. SMTP 465/587 used local TLS sinks and throwaway credentials.

The internal journal weekly_email delivery gate remains false until an accepted
authoritative JournalSource is wired; no local environment switch can enable it.
Preference writes still persist with 204 and missing-token requests refuse with
403. Live Game publication/delivery is not claimed; its pending/unavailable states
are covered. The supplied negative catalogue remains inactive and unpublished.

## Rust acceptance

Running with PORTAL_TEST_DATABASE_URL set to the private disposable test database:

```sh
cd portal-rs
cargo fmt --all --check && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked
```

Serve and worker were stopped after browser completion to release private test
listener ports. The launcher retains the database until acceptance completes.
Final Rust pass/fail/ignored and identified PostgreSQL counts will be appended.

Raw traces, cookies, captured mail, keys and database URLs remain private in
scratch or ignored artifacts. Only sanitized evidence is committed. No workflow,
Rust dependency or lockfile changes were made. No pull request was opened.
