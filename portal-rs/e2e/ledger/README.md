# Portal E2E coverage ledger

From the repository root, use the disposable PostgreSQL setup in the [browser qualification README](../README.md). With the local administrator URL available as `PORTAL_E2E_ADMIN_DATABASE_URL`, run:

```sh
CARGO_BUILD_JOBS=2 portal-rs/e2e/ledger/llvm-cov.sh
```

This entry point rebuilds the route and form inventories, runs all four Playwright projects against the same instrumented portal, and refreshes the request, form and LLVM reports. Pass `--gate` to also fail on every unhit request or form row. Playwright failures retain their usual nonzero exit status. Commit the regenerated inventories and aggregate reports when refreshing the baseline.

Raw Playwright results, traces, per-worker JSONL and LLVM profiles stay in the private run directory printed by the command. The JSONL files are isolated per run and separate from the Playwright output directory that gets cleared at startup.

The recorder installs in Playwright workers before context creation. Browser context events include popups and pages opened by specs. Playwright's pinned in-process transport exposes API request events for both standalone and browser-associated contexts, including individual redirect hops. Check these hooks when upgrading Playwright. Each request is associated with the active Playwright test, including its setup and teardown hooks.

The recorder matches the portal origin against `inventory.json`, preferring literal segments over parameters and catchalls. Paths absent from inventory and requests to other origins have a null route pattern. Requests with no HTTP response have a null status and are excluded from coverage. The inventory has two GET patterns with the same path shape for Bazaar items. The resolver uses inventory order for that tie.

The recorder uses the harness's disposable database connection to resolve the session cookie sent on each request. Authenticated accounts are represented by sorted role keys, with `player` for accounts without a staff role. Requests without an authenticated session are `anonymous`. Only inventory patterns, template-defined button names and values, actor labels, status codes and Playwright metadata are written.

Form rows group template locations by action pattern, method, and submitter name and value. An unnamed submitter gets its own row. The extractor expands template conditionals and uses the owning renderers' paths for shared templates and implicit actions. Some form destinations are missing from the provisional route inventory. Those forms still have their own rows. The JSON identifies their mapping, when available, and includes each template location.

Form evidence is based on query or form-encoded fields sent to the matching action. It includes deliberate API form probes. An unnamed submitter is inferred from declared form fields. An ordinary navigation with a matching form query can produce the same evidence as a browser submission. Dynamic ranking pagination buttons are identified by the submitted page relative to the referring page.

Reports list observed actor and status combinations and their projects. Success means 2xx or 3xx. Refusal means 400, 401, 403, 404, or 422. Row IDs use a deterministic number derived from the row's identity and survive report reruns and insertion of unrelated rows. The gate writes both reports and returns nonzero if any route or form row has no hit, listing each missing row.
