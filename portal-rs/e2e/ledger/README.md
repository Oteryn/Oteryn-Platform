# Portal E2E coverage ledger

Run the browser suite using the disposable PostgreSQL and HTTPS setup in the [browser qualification README](../README.md). From `portal-rs/e2e`, regenerate the form inventory before starting the full run and then generate the coverage report.

```sh
python3 ledger/forms.py
node run.mjs /absolute/scratch/env.json test
node ledger/report.mjs
node ledger/report.mjs --gate
```

Playwright clears its output directory at the start of a run. Use a single invocation for all four projects. Every worker appends to its own `test-results/ledger/<project>-<worker>.jsonl` file. The JSONL is local. Commit the generated form inventory and the two aggregate reports when refreshing the baseline.

The recorder installs in Playwright workers before context creation. Browser context events include popups and pages opened by specs. Playwright's pinned in-process transport exposes API request events for both standalone and browser-associated contexts, including individual redirect hops. Check these hooks when upgrading Playwright. Each request is associated with the active Playwright test. The shared fixture awaits lookups and form capture during teardown. Capture errors fail the attempt with a fixed diagnostic and prevent report generation.

The recorder matches the portal origin against `inventory.json`, preferring literal segments over parameters and catchalls. Paths absent from inventory and requests to other origins have a null route pattern. Requests with no HTTP response have a null status and are excluded from coverage. The inventory has two GET patterns with the same path shape for Bazaar items. The resolver uses inventory order for that tie.

The recorder labels the supplied credential as `session`, `bearer`, `authorization`, or `none`. An Authorization header takes precedence over a cookie for attribution. Its principal stays unresolved because the recorder does not validate that credential. Session lookup can resolve a player or sorted staff role keys in the disposable database, identify an anonymous session, or leave the principal unresolved when expiry or revocation removes the session. The ledger describes request credentials and lookup evidence. HTTP statuses describe the route outcome. It writes inventory patterns, static submitter names and values, credential and principal labels, status codes and Playwright metadata.

Form rows group template locations by action pattern, method, and submitter name and value. An unnamed submitter gets its own row. Calculator forms also use their static `calculator` operation in the row identity, so each submission credits one operation. The extractor expands template conditionals and uses the owning renderers' paths for shared templates and implicit actions. Some form destinations are missing from the provisional route inventory. Those forms still have their own rows. The JSON identifies their mapping, when available, and includes each template location.

Form evidence requires Chromium to identify a renderer-initiated form navigation for the same document loader, method and destination. API calls and ordinary navigations supply route evidence. Redirect hops also supply route evidence. Only the initial form submission supplies form evidence. Dynamic ranking pagination buttons are identified by the submitted page relative to the referring page.

The report joins requests to the same project, test, worker and retry in Playwright's `test-results/results.json`. A request receives credit only when that test attempt passed and was expected to pass. Playwright records the checkout revision, clean-tree status, and a unique run ID in the results and on each request. The report validates these values before using evidence and can regenerate reports from saved captures without Git.

Reports list observed actor and status combinations with passing test and project provenance in JSON. The two status columns group observed 2xx or 3xx responses and selected 4xx responses (400, 401, 403, 404, and 422). Redirects can represent either acceptance or denial at the application layer. Row IDs use a deterministic number derived from the row's identity and survive report reruns and insertion of unrelated rows. The gate writes both reports and returns nonzero if any route or form row has no hit, listing each missing row.

## Checked-in baseline and branch delta

Regenerate the form inventory and commit the candidate, then capture both suites before generating the reports. Rebuild the baseline from the standard desktop Polish suite, then capture the branch cases separately in both review projects. The roster-filter case uses the `ledger-unhit` prefix in its title so it is included with the branch cases.

```sh
node run.mjs /absolute/scratch/env.json test --project=pl-1440 --grep-invert ledger-unhit
mv test-results /absolute/scratch/baseline-results
node run.mjs /absolute/scratch/env.json test --project=pl-1440 --project=en-390 --grep ledger-unhit
node ledger/report.mjs --results-dir /absolute/scratch/baseline-results
node ledger/report.mjs --delta
```

The current-rule [baseline](LEDGER.md) uses browser form submissions and passing attempts. The [delta](DELTA.md) lists each additional row with its passing test and project. [Remaining coverage gaps](GAPS.md) explains each unhit delta row. A report generated under older rules must be rebuilt before generating the delta.
