# Authorization matrix

Regenerate the route inventory with `python3 authz/generate.py` from
`portal-rs/e2e`. Use `--check` to verify the committed output. The inventory
records the route-registration source for each listener, URL pattern and HTTP
method. The actor definitions use permission defaults from the Rust RBAC catalog.

## Database assertions

Refusal assertions preserve all existing rows in `public.audit_log` and
require the expected list of newly appended refusals. The non-audit check
fingerprints every regular application table in a repeatable-read transaction.
It has two narrowly scoped bookkeeping exclusions.

- On `public.web_sessions`, exclude `last_seen_at`, `updated_at`, and
  `idle_expires_at` only for the session presented on the tested request.
  Other fields and sessions stay in the fingerprint.
  [Session middleware](../../crates/web/src/session.rs#L480)
- On `public.referral_activity`, check the tested session's account row against
  the session update. An activity
  change requires an advance in that session's last-seen timestamp.
  `last_activity` must equal the greater of the previous activity and the
  refreshed session timestamp. `updated_at` must follow the refresh. Existing
  row identity and other columns remain strict. All other accounts' rows stay
  in the fingerprint.
  [Session activity trigger](../../migrations/20261118130101_fb_referral_activity_flag.sql#L58)

Run the browser and private harnesses on separate disposable local databases
with distinct listener ports. Once the browser launcher reports readiness,
stop its `serve` and `worker` children with SIGINT and keep the launcher
running. The launcher owns the disposable database and browser HTTPS proxy.
Restart the same portal binary with the same browser environment and an unused
private listener port. The example port below is separate from the browser
producer destinations and the private harness's listeners.

```sh
PORTAL_E2E_BINARY=/absolute/path/to/oteryn-portal \
node authz/serve-isolated.mjs /absolute/path/to/browser/env.json 18447
```

The browser proxy still reaches the real portal, and the independent private
harness serves the TLS and service-token checks. This keeps periodic producer
and outbox writes out of the observation window while retaining the affected
tables in the assertion. The launcher also enables the provider webhook with
an ephemeral signing key and starts the direct native TLS listener on 18444
by default. Pass a third argument to use a different native port and set
`PORTAL_E2E_BROWSER_NATIVE_ORIGIN` in the test environment.

Suspend the private harness's `worker` child with SIGSTOP after the harness
reports ready. The gateway checks compare its database across requests.
Resume the child with SIGCONT before stopping the private harness.

## Run

From `portal-rs/e2e`, use the browser harness's private environment file and
the private harness's report path. Supply the built portal executable for the
seed CLI. Use the existing harness launchers described in [the E2E README](../README.md).

```sh
PORTAL_E2E_BINARY=/absolute/path/to/oteryn-portal \
PORTAL_E2E_HARNESS_REPORT=/absolute/path/to/private-harness/report.json \
node run.mjs /absolute/path/to/browser/env.json test --config authz/all.config.mjs
```

The command exercises all four projects. The narrower configs run public and
private listener checks (`playwright.config.mjs`), protected reads and editorial
workflows (`protected.config.mjs`), account sessions, support and content
(`sessions.config.mjs`), wiki editorial workflows (`wiki.config.mjs`), browser
credentials (`credentials.config.mjs`), and termination
(`termination.config.mjs`). Use
separate disposable databases for concurrent project runs because refusal
comparisons include every application table in the database.

To exercise alternative registrations for the disabled coin checkout and
marketplace integrations, start another disposable browser profile with
`PORTAL_E2E_AUTHZ_UNAVAILABLE=1` when invoking `serve-isolated.mjs`. Run
`authz/checkout-unavailable.config.mjs` against that profile in all four
projects.

## Executable coverage

The inventory records 672 listener/path/method entries. The executable matrix
covers 613 of these entries for each of the 19 actors, for 11,647 cells per
project. The fixtures expect 7,538 allowed and 4,109 refused cells. The
browser cases cover public reads, staff reads and mutations, player support,
sessions and recovery keys, and the missing signatures on both payment
webhooks. The isolated private listener covers Bazaar, account closure, and
league mTLS. Set `PORTAL_E2E_BROWSER_PRIVATE_ORIGIN` when using another private
port. The mTLS scope assignment and revocation probes also present a CA-valid certificate
assigned to a different route identity and expects HTTP refusal. The native
admission listener checks its service token independently.

[Local limitations](local-limitations.json) records the other 59 entries.
Each entry identifies its source and the condition preventing a local matrix
case, including pre-authorization writes that conflict with the strict
refusal invariant.
