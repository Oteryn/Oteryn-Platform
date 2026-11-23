# Browser accessibility and performance budgets

Use the prerequisites from [Portal browser qualification](../README.md) and a
disposable local PostgreSQL 18.6 administrator connection. The launcher uses
the existing cross-repository harness, seed CLI and Chromium projects.

```sh
cd portal-rs/e2e
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts --cache /tmp/portal-browser-npm
node quality/run.mjs
```

The command above runs the complete quality matrix. It reads the built-in
role catalogue from the migrated database and runs an independent case for
every staff role, anonymous visitor and player in all four projects. Each
case walks the entire source and linked route inventory with that actor's
session and permissions.

Set `PORTAL_E2E_ADMIN_DATABASE_URL` privately before running. The launcher
builds the portal with two Cargo build jobs. It accepts `PORTAL_E2E_BINARY`
for a prebuilt portal. Standard Playwright filters such as `--project=en-390`
and `--grep=platform_admin` apply. For single-route diagnosis,
`PORTAL_QUALITY_ROUTE` accepts comma-separated exact source patterns, such as
`/{lang}/,/account`.

For separate Playwright shards on one host, build the portal once and set
`PORTAL_E2E_BINARY` to that binary. Give each shard distinct ports and report
paths. This example runs four shards sequentially. The same four invocations
can run on separate hosts with their own disposable PostgreSQL instances.

```sh
for shard in 1 2 3 4; do
  PORTAL_QUALITY_HTTPS_PORT=$((18440 + shard)) \
  PORTAL_E2E_HTTP_PORT=$((18080 + shard)) \
  PORTAL_E2E_INTERNAL_PORT=$((18540 + shard)) \
  PORTAL_E2E_ADMISSION_PORT=$((18090 + shard)) \
  PORTAL_E2E_GATEWAY_PORT=$((18100 + shard)) \
  PLAYWRIGHT_JSON_OUTPUT_FILE="test-results/quality-shard-$shard.json" \
  node quality/run.mjs --shard="$shard/4" --output="test-results/quality-shard-$shard" || exit 1
done
```

Use the complete set of shard reports to verify the matrix. Each case's
`quality-inventory` attachment identifies its actor, locale and viewport
width. Count `audited` outcomes from completed cases whose Playwright result
passed. `already audited` entries reference another audit in the same case.
A complete crawl has one passing case per actor and project and no route
filter. The per-case inventory is also written during the crawl, and the
terminal reports progress after every 25 samples.

Set `PORTAL_QUALITY_PAINT_TRACE=1` to record Chromium LCP candidate events,
long tasks and rendering phases before the final paint. Each document's
inventory entry also identifies the candidate element and records navigation
TTFB and resource timing, including render-blocking stylesheets. The trace
summary omits screenshots, raw trace events and request headers.

The launcher regenerates the existing Rust route inventory in scratch. The
tests use GET registrations, navigation registry destinations and same-origin
links rendered by the portal. Each pattern has at most three samples per
actor and project. The seed CLI creates verified accounts and a local world.
The existing registration helper enables the disposable password-range
fixture so the real registration form is available. The browser creates
editorial content, support tickets and reports through their real forms.
All staff go through the real MFA challenge. A separate platform
administrator publishes the shared public editorial fixtures for each
worker so sharding preserves the public content coverage. Source
patterns that require a resource absent from the harness use a representative
missing identifier, with the resulting status recorded. Each actor gets a
fresh browser context for every measured destination. A real HTTP discovery
request classifies the destination before Chromium opens it. Redirect
aliases and identical shared refusal screens retain route evidence and
reuse the audit of the same document. Game-dependent views retain the
harness's cold or unavailable responses when their authority is absent.

All actors run in Polish and English at 1440 × 900 and 390 × 900.
Axe checks WCAG 2 A and AA with zero serious or critical violations. The suite
also checks contrast, form labels, landmarks, and bypass navigation. It
exercises the skip link, menu disclosures, account dialog and form focus with
Chromium keyboard events.

| Per-navigation budget | Limit |
| --- | ---: |
| Largest Contentful Paint | < 2,500 ms |
| Cumulative Layout Shift | < 0.1 |
| JavaScript response bodies | < 180,000 bytes |
| CSS response bodies | < 120,000 bytes |
| Each response body | < 1,000,000 bytes |

Transfer limits use the encoded response body size from Chromium on a fresh
context. The CLS calculation uses the largest layout-shift session window
and excludes recent input. Successful static assets require reusable cache
headers and assets with a content hash require `immutable`.

The local `test-results/quality-results.json` report contains a
`quality-inventory` attachment for each actor and project. It records each
requested path, final document path, status and audit outcome, with metrics
and axe findings for each audited document. The launcher tears down its
services and disposable database. The harness scratch directory is private
and contains throwaway credentials. Keep it local.
