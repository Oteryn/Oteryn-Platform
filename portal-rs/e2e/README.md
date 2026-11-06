# Portal browser qualification

Node 22+, OpenSSL, the repository Rust toolchain, PostgreSQL 18.6, and an installed Chromium are required. No browser download is performed. The suite has four projects: Polish and English, each at 1440 × 900 and 390 × 900. Retries are disabled. Traces and screenshots remain local and can contain throwaway authentication material; do not publish them.

From the repository root in the supplied environment:

```sh
source /workspace/oteryn-platform-env/env.sh
export PATH=/workspace/portal-e2e-night/node-v22.16.0-linux-x64/bin:$PATH
/workspace/oteryn-platform-env/start.sh
cd portal-rs/e2e
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts --cache /tmp/portal-browser-npm
# Supply a LOCAL disposable PostgreSQL administrator connection through the environment.
# Do not put credentials in this file or commit them.
node local.mjs
```

`local.mjs` requires `PORTAL_E2E_ADMIN_DATABASE_URL`. It creates two uniquely named databases, generates throwaway keys and a local CA/server certificate, builds with `cargo build --locked -p oteryn-portal`, migrates, and starts the portal, worker, TLS SMTP sink and HTTPS proxy on loopback. It enables module flags found in Rust source and reports its scratch directory. Port 465 must be available and bindable by the local user. Native game authority/evidence is never fabricated. Ctrl-C stops the task processes and drops only its two databases; the scratch directory is retained for local investigation and should be removed after inspecting the report.

In a second shell, using the scratch path printed by that process:

```sh
source /workspace/oteryn-platform-env/env.sh
export PATH=/workspace/portal-e2e-night/node-v22.16.0-linux-x64/bin:$PATH
cd /workspace/Oteryn-Platform/portal-rs/e2e
CHROMIUM_PATH=/usr/bin/chromium node run.mjs /absolute/scratch/env.json test
```

That is the exact browser command; it runs every project. `--project=pl-390` can be appended for diagnosis. The existing Chromium path can be overridden using `CHROMIUM_PATH`; never run a browser installation command when Chromium is present.

The supplied environment exposes Node through a container wrapper. A native Node executable must be on PATH when running these loopback services and Chromium. Qualification used Node 22.16.0 unpacked outside the repository at `/workspace/portal-e2e-night/node-v22.16.0-linux-x64/bin`:

```sh
export PATH=/workspace/portal-e2e-night/node-v22.16.0-linux-x64/bin:$PATH
```

For an already running isolated portal, set `PORTAL_E2E_ORIGIN` and `PORTAL_E2E_DATABASE_URL` and run `npm test`. Fixtures refuse non-local connections and database names without the `portal_e2e_` prefix. Registration seeds the same synthetic password-breach corpus used by the identity crate's tests; browser proof of work, CSRF, quotas and verification mail remain active. Registration waits for the real quota window instead of changing limits or deleting quota rows. The mail sink is available only on local `/__mail`; it is a harness endpoint, never a portal route.

Item-bazaar, game-login-history, journal-report, dashboard and season-pass startup adapters document missing accepted game authority. Those cases assert exactly HTTP 503, no-store and no account-data disclosure; they do not prove a successful game-backed flow.

The suite includes public navigation/accessibility checks, identity lifecycle, account surface reads/CSRF, ticket creation/reply/foreign-owner refusal, character requests, recovery-key management, player reports, party refusal and staff bootstrap/MFA/navigation. The current report explicitly lists unimplemented flows and qualification failures; this suite is not yet a complete proof of every flow in the requested acceptance matrix. Tests retain strict success assertions for required flows. A game-dependent refusal must be explicitly documented and asserted before it can count as qualified.

Workspace acceptance uses the generated `PORTAL_TEST_DATABASE_URL`. Export it from the private scratch JSON without printing it, then run:

```sh
cd /workspace/Oteryn-Platform/portal-rs
RUST_TEST_THREADS=1 cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
RUST_TEST_THREADS=1 cargo test --locked
```

The thread bound limits concurrent PostgreSQL schemas; it does not skip tests. Read [REPORT.md](REPORT.md) for observed outcomes and [ENVIRONMENT.md](ENVIRONMENT.md) for source-derived variable names.
