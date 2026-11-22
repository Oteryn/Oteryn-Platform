# Portal E2E coverage ledger

Run the browser suite using the disposable PostgreSQL and HTTPS setup in the [browser qualification README](../README.md). From `portal-rs/e2e`, regenerate the form inventory before starting the full run and then generate the coverage report.

```sh
python3 ledger/forms.py
node run.mjs /absolute/scratch/env.json test
node ledger/report.mjs
node ledger/report.mjs --gate
```

Playwright clears its output directory at the start of a run. Use a single invocation for all four projects. Every worker appends to its own `test-results/ledger/<project>-<worker>.jsonl` file. The JSONL is local. Commit the generated form inventory and the two aggregate reports when refreshing the baseline.

The recorder installs in Playwright workers before context creation. Browser context events include popups and pages opened by specs. Playwright's pinned in-process transport exposes API request events for both standalone and browser-associated contexts, including individual redirect hops. Check these hooks when upgrading Playwright. Each request is associated with the active Playwright test, including its setup and teardown hooks.

The recorder matches the portal origin against `inventory.json`, preferring literal segments over parameters and catchalls. Paths absent from inventory and requests to other origins have a null route pattern. Requests with no HTTP response have a null status and are excluded from coverage. The inventory has two GET patterns with the same path shape for Bazaar items. The resolver uses inventory order for that tie.

The recorder looks up the session cookie sent on each request in the disposable database. It records sorted staff role keys or `player` for an authenticated account, and `anonymous` for a request with no session cookie or an existing anonymous session. An expired, revoked, or otherwise unresolved session is labeled `unknown` because lookup can race a session mutation. Only inventory patterns, static submitter names and values, actor labels, status codes and Playwright metadata are written.

Form rows group template locations by action pattern, method, and submitter name and value. An unnamed submitter gets its own row. The extractor expands template conditionals and uses the owning renderers' paths for shared templates and implicit actions. Some form destinations are missing from the provisional route inventory. Those forms still have their own rows. The JSON identifies their mapping, when available, and includes each template location.

Form evidence requires Chromium to identify a renderer-initiated form navigation for the same document loader, method and destination. API calls and ordinary navigations supply route evidence. Redirect hops also supply route evidence. Only the initial form submission supplies form evidence. Dynamic ranking pagination buttons are identified by the submitted page relative to the referring page.

The report joins requests to the same project, test, worker and retry in Playwright's `test-results/results.json`. A request receives credit only when that test attempt passed and was expected to pass. Keep the JSON results and recorder files from the same invocation.

Reports list observed actor and status combinations with passing test and project provenance in JSON. Success means 2xx or 3xx. Refusal means 400, 401, 403, 404, or 422. Row IDs use a deterministic number derived from the row's identity and survive report reruns and insertion of unrelated rows. The gate writes both reports and returns nonzero if any route or form row has no hit, listing each missing row.

## Checked-in baseline

The checked-in reports cover the completed `pl-1440` project. It ran 460 tests, with 457 passing and three failing on the calculator page's color contrast. Its request evidence includes all 460 tests. The four-project run was still in progress when this baseline was published.

## Branch coverage delta

The [baseline delta](DELTA.md) lists additional rows exercised by passing branch tests. Keep the historical checked-in baseline for comparison. Run the affected projects in one invocation with the default output paths, then run `node ledger/report.mjs --delta` on that same revision to generate the delta.
