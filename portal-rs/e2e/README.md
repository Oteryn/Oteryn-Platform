# Portal browser qualification

Node 22+, Python 3 (pinned negative fixture generation), OpenSSL, the repository Rust toolchain, PostgreSQL 18.6, and an installed Chromium are required. No browser download is performed. The suite has four projects: Polish and English, each at 1440 × 900 and 390 × 900. Retries are disabled. Traces and screenshots remain local and can contain throwaway authentication material; do not publish them.

From the repository root in the supplied environment:

```sh
portal_e2e_node_bin="$(dirname "$(command -v node)")"
source /workspace/oteryn-platform-env/env.sh
export PATH="$portal_e2e_node_bin:$PATH"
/workspace/oteryn-platform-env/start.sh
cd portal-rs/e2e
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts --cache /tmp/portal-browser-npm
# Supply a LOCAL disposable PostgreSQL administrator connection through the environment.
# Do not put credentials in this file or commit them.
node local.mjs
```

`local.mjs` requires `PORTAL_E2E_ADMIN_DATABASE_URL`. It creates two uniquely named databases, generates throwaway keys and a local CA/server certificate, builds with `cargo build --locked -p oteryn-portal`, migrates, and starts the portal, worker, TLS SMTP sink and HTTPS proxy on loopback. It enables module flags found in Rust source and reports its scratch directory. Ports 465 and 587 must be available and bindable by the local user. Native game authority/evidence is never fabricated. Ctrl-C stops the task processes and drops only its two databases; the scratch directory is retained for local investigation and should be removed after inspecting the report.

In a second shell, using the scratch path printed by that process:

```sh
portal_e2e_node_bin="$(dirname "$(command -v node)")"
source /workspace/oteryn-platform-env/env.sh
export PATH="$portal_e2e_node_bin:$PATH"
cd /workspace/Oteryn-Platform/portal-rs/e2e
CHROMIUM_PATH=/usr/bin/chromium node run.mjs /absolute/scratch/env.json test
```

That is the exact browser command; it runs every project. `--project=pl-390` can be appended for diagnosis. The existing Chromium path can be overridden using `CHROMIUM_PATH`; never run a browser installation command when Chromium is present.

The environment script prepends a container wrapper for Node. The commands above
retain the native Node executable already on PATH before sourcing that script, so
loopback services and Chromium run together. Use a native Node 22+ installation.

For an already running isolated portal, set `PORTAL_E2E_ORIGIN` and `PORTAL_E2E_DATABASE_URL` and run `npm test`. Fixtures refuse non-local connections and database names without the `portal_e2e_` prefix. Registration seeds the same synthetic password-breach corpus used by the identity crate's tests; browser proof of work, CSRF, quotas and verification mail remain active. Registration waits for the real quota window instead of changing limits or deleting quota rows. The mail sink is available only on local `/__mail`; it is a harness endpoint, never a portal route.

The local launcher supplies a disposable Bazaar Game producer and the real private transport. Character/item Bazaar and account listing reads require HTTP 200, no-store, the shared shell and CSRF refusal for every rendered mutation. Missing listing references require HTTP 404; malformed UUIDs require HTTP 400. Both retain no-store and the shared shell without account disclosure. The dedicated Bazaar spec proves funded bids, delivery, seller credit and Game-refusal behavior against synthetic fixtures; it does not qualify a production Game integration.

Game-login-history, journal-report, dashboard and season-pass startup adapters still document missing accepted game authority. Those cases assert exactly HTTP 503, no-store and no account-data disclosure; they do not prove a successful game-backed flow.

The suite includes public navigation/accessibility checks, identity lifecycle, account surface reads/CSRF, ticket creation/reply/foreign-owner refusal, character requests, recovery-key management, player reports, party refusal and staff bootstrap/MFA/navigation. The report distinguishes the completed composition flows from broader product workflows outside this task. Tests retain strict success assertions for required portal flows. A game-dependent refusal must be explicitly documented and asserted before it can count as qualified.

After the browser suite, stop the launcher’s `serve` and `worker` child processes
with SIGINT, keeping `local.mjs` itself alive until Rust acceptance completes. The
composed-binary Rust test binds the same default private admission port; running
both servers together causes a port conflict. Stopping the launcher instead would
drop its disposable test database.

Workspace acceptance uses the generated `PORTAL_TEST_DATABASE_URL`. Export it from the private scratch JSON without printing it, then run:

```sh
cd /workspace/Oteryn-Platform/portal-rs
RUST_TEST_THREADS=1 cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
RUST_TEST_THREADS=1 cargo test --locked
```

The thread bound limits concurrent PostgreSQL schemas; it does not skip tests. Read [REPORT.md](REPORT.md) for observed outcomes and [ENVIRONMENT.md](ENVIRONMENT.md) for source-derived variable names.

The local sink serves implicit TLS on 465 for registration/reset and STARTTLS on 587
for the existing notification/e-mail-change sender, sharing the captured mailbox.
Certificate verification and authenticated SMTP stay enabled. E-mail-change delivery
runs on the existing one-minute worker schedule; its browser case waits for that
schedule. Public navigation allows enough time to analyze every HTML refusal with
the same shell, focus, overflow and accessibility assertions as successful pages.
Cold projections require exactly 503 and no-store. Expected browser network notices
for those exact 503 destinations are recorded separately from unexpected console or
script errors. Missing routes and arbitrary failures remain failures.

Registration allows 90 seconds for the real browser proof of work. Native form
actions allow 30 seconds for password hashing and navigation. These waits retain
the exact destination assertions, production proof difficulty and zero retries.


## Native world source and expected refusals

`local.mjs` generates the CATALOG_EXPORT_V1 negative export in scratch using the
exact reference producer from Game
`d34807f5bd234ee8318e57ba454ba39be0df113d`, with its byte sidecar and explicit
provenance fence. The fixture's internal source revision is `a2a5da955dd8f580c9e768c8ac6a741db388cb22`;
that is distinct from the producer evidence pin. See
[`SOURCE.md`](../crates/world/tests/fixtures/catalog-fd4be4e2/SOURCE.md).
No generated artifact or sidecar is committed. Use `PORTAL_E2E_CATALOG_FILE`
for another local export, supply `OTERYN_WORLD_CATALOG_SHA256` independently,
and set the five
`OTERYN_WORLD_CATALOG_*` provenance values explicitly to qualify a different
candidate. The runtime never fetches, reads raw Game layout, seeds world rows,
activates a candidate or grants visibility. `PORTAL_E2E_GAME_DIR` is superseded by
the offline export contract and is not consumed. Today uses a fixed UUIDv7 in the
harness; no `world_daily` producer is fabricated.

Run the focused browser qualification in all four locale/viewport projects.
A standalone public sweep also needs the existing synthetic registration fixture
normally prepared by earlier identity/account specs. Use its existing helper on
the disposable database; this does not create a world catalogue or Today snapshot:

```sh
node --input-type=module -e 'import fs from "node:fs"; import {seedPasswordRange,password} from "./helpers.mjs"; Object.assign(process.env,JSON.parse(fs.readFileSync(process.argv[1],"utf8"))); await seedPasswordRange(password());' /absolute/scratch/env.json
node run.mjs /absolute/scratch/env.json test specs/world-read.spec.mjs specs/public.spec.mjs
```

Expected native results are deliberate fail-closed, not successful data reads:

| Pages | Expected | Exact blocker/evidence |
| --- | --- | --- |
| `/{pl,en}/wiki`, `/wiki/search`, entity lists | 503 + no-store + shared shell | All eight export capabilities are `unsupported/unknown`, with no entities: [input fixture](../crates/world/tests/fixtures/catalog-fd4be4e2/unsupported-native-input.json#L9). The lane explicitly refuses promotion of native capability payloads: [lane notes:38–62](https://github.com/Oteryn/Oteryn-Game/blob/fd4be4e214ef33ad65d3ce465dab7160377e62f5/docs/reference/CATALOG-CONTRACT-NOTES.md#L38). There are no approved catalogue names/stats/relations to render. |
| `/{lang}/atlas` | 503 + no-store + shared shell | Needs a **Game-side atlas export**. At Game `d34807f5`, [`content/world/assets/catalog.json:1`](https://github.com/Oteryn/Oteryn-Game/blob/d34807f5bd234ee8318e57ba454ba39be0df113d/content/world/assets/catalog.json#L1) contains `assets:[]`; CATALOG_EXPORT_V1 has no map capability. No terrain/B3 substitute is rendered. |
| `/{lang}/atlas/delivery` | 503 + no-store + shared shell | Item/creature/loot/quest capabilities remain unsupported; no public active catalogue or accepted payloads. |
| `/{lang}/today/data`, `/today/widget` | 503 + no-store (JSON / HTML fragment) | G-12 is [PROPOSED, changes needed](../../docs/contracts/OTERYN_V2_WORLD_DAILY_FAMILY_CONTRACT_PROPOSAL.md#L1), with no accepted Game producer. No ingest evidence or supported catalogue names are invented. |
| `/{lang}/stats/world` without query | 200 + no-store + shared shell when registered worlds exist | Real registry picker; empty/unavailable registry still returns 503. |
| `/{lang}/stats/world?world=…&start=…&end=…` | 503 + no-store | Existing privacy/visibility policy is unavailable; the picker grants no metrics access. |

G-2 is resolved at lane `fd4be4e214ef33ad65d3ce465dab7160377e62f5` (re-pin on
merge): hidden identities never enter the export; an explicit `visibility` member
must be the string `public`. Independent admin approval remains required. The
For an intentional transport failure probe, set
`PORTAL_E2E_CATALOG_EXPECT_UNAVAILABLE=1` alongside the URL; a healthy URL serving
the pinned negative artifact still creates one inactive candidate. The
harness enables the real publication switch, but the lane's unsupported negative
fixture still cannot publish any category. `/wiki/areas` awaits the scheduled
additive `area` export category. Deaths, online, stats/guilds and wiki/places are
outside this task and retain their existing behavior.

Atlas flag aliases are centralized: `OTERYN_ATLAS_DELIVERY_ENABLED=true` aliases
`PORTAL_ATLAS_DELIVERY_ENABLED=1`; `OTERYN_WORLD_ENABLED=true` aliases
`PORTAL_WORLD_ENABLED=1`. If both aliases are supplied, **both must enable** the
gate; false, invalid or conflicting values refuse. Viewer enablement still needs
`OTERYN_ATLAS_ENABLED=true` and `OTERYN_ATLAS_DIRECTORY`.
