# Portal API contract tests

Use the disposable PostgreSQL 18.6 connection and install the Node dependencies
as described in [the E2E README](../README.md). Build the portal from the checkout,
then run the request suite and the four browser projects together.

```sh
cd portal-rs
CARGO_BUILD_JOBS=2 cargo build --locked -p oteryn-portal
cd e2e
node api/run.mjs
```

The runner starts `harness.mjs`, uses its seed CLI and generated certificates,
and tears down the task database on exit. `PORTAL_E2E_ADMIN_DATABASE_URL` must
target a disposable loopback instance. `PORTAL_E2E_BINARY` can select the
binary from an external target directory. Arguments after `run.mjs` are
Playwright options, such as `--project=api`. The authenticated OAuth flow
runs under the existing Polish and English desktop and mobile projects.
All HTTP assertions use Playwright request contexts.

Tests send fixture bytes without re-encoding exact wire vectors. They compact
the Store fixtures, which are formatted for documentation, before transport.
For generated ticket and ledger values, tests validate the field independently
then compare the remaining wire bytes against the fixture. The coin suite seeds
an opening credit in the disposable database because the seed CLI supports
accounts but has no credit command. Transfers, reservations, settlement, release,
and replay all go through the real mTLS listener.

The harness generates credentials for each run. The browser test disables
traces and screenshots, and the runner retains the credentials only in
memory and in its private temporary directory for the duration of the run.

## Candidate findings and coverage gaps

- With the configured certificate and body `{}`, native evidence, native
  runtime status and native scope assignment answered 400 with an HTML body.
  The candidate wires specify an empty refusal. The strict 401 body checks
  for unrelated peers pass. The three 400 body discrepancies are recorded
  here for the contract owners.
- The admission harness verifies service authentication and rate limiting.
  Successful admission grants, protocol-3 recovery and locator publication
  require a live authority chain and signing custody.
- Profile B, enforcement, analytics and journal families exercise their
  default-off refusal. Publication and reconciliation with configured
  producers, authenticated admin-agent exchange, Bazaar authority, and
  successful Character command delivery are outside the current suite.
- Store delivery checks framing and empty or missing outcomes. A fulfilled
  purchase and downstream delivery receipt need a catalog import and matching
  operation. The webhook test uses a signed ignored event, with no paid order.
- The suite covers only the routes represented by the request cases. It does
  not provide the generated route and actor completeness ledger from #2002.
