# Portal browser qualification

Node 22+, Python 3 (pinned negative fixture generation), OpenSSL, the repository Rust toolchain, PostgreSQL 18.6, and an installed Chromium are required. No browser download is performed. The suite has four projects: Polish and English, each at 1440 × 900 and 390 × 900. Retries are disabled. Traces and screenshots remain local and can contain throwaway authentication material; do not publish them.

From the repository root, with a native Node 22+ on `PATH` and a throwaway local
PostgreSQL 18.6 already running (nothing outside the repository is sourced):

```sh
cd portal-rs/e2e
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts --cache /tmp/portal-browser-npm
# Supply a LOCAL disposable PostgreSQL administrator connection through the environment.
# Do not put credentials in this file or commit them.
node local.mjs
```

`local.mjs` requires `PORTAL_E2E_ADMIN_DATABASE_URL`. It creates two uniquely named databases, generates throwaway keys and a local CA/server certificate, builds with `cargo build --locked -p oteryn-portal`, migrates, and starts the portal, worker, TLS SMTP sink and HTTPS proxy on loopback. It enables module flags found in Rust source and reports its scratch directory. Ports 465 and 587 must be available and bindable by the local user. Native game authority/evidence is never fabricated. Ctrl-C stops the task processes and drops only its two databases; the scratch directory is retained for local investigation and should be removed after inspecting the report.

In a second shell, using the scratch path printed by that process:

```sh
cd portal-rs/e2e
CHROMIUM_PATH=/usr/bin/chromium node run.mjs /absolute/scratch/env.json test
```

That is the exact browser command; it runs every project. `--project=pl-390` can be appended for diagnosis. The existing Chromium path can be overridden using `CHROMIUM_PATH`; never run a browser installation command when Chromium is present.

Use a native Node 22+ installation, so loopback services and Chromium run together.

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
cd portal-rs
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


## Cross-repository harness (Game server + client + portal-rs + PostgreSQL)

One command starts portal-rs for an external E2E harness. It needs Node 22+,
OpenSSL, the repository Rust toolchain and a throwaway **local** PostgreSQL 18.6
administrator URL; everything else is generated per run:

```sh
set +x
export PORTAL_E2E_ADMIN_DATABASE_URL=...   # local PostgreSQL only; never commit or print it
portal-rs/e2e/start.sh --seed-plan portal-rs/e2e/seed-plan.example.json
```

`start.sh` checks the tools, installs the pinned `pg` client from
`package-lock.json` when missing (no browser, no install scripts) and runs
`harness.mjs`, which:

1. reads the test-only settings in [`test.env`](test.env) (flags, authorities,
   ports; no secrets; a variable already set in the calling shell overrides a
   name listed there);
2. creates a fresh `portal_e2e_*` database, builds `oteryn-portal`, copies the
   binary into the output directory and runs `migrate`;
3. generates a throwaway CA, an internal-listener server certificate
   (`localhost`, `127.0.0.1`) and one client certificate per internal Game
   identity, each with a single DNS SAN `<identity>.game.e2e.oteryn.test`, and
   binds each SAN to its own `*_IDENTITY` / `*_IDENTITIES` / `*_SAN` variable
   (no overlap, so the shared identity check keeps every route available);
4. starts `serve` (public listener plus the internal mTLS listener), `worker`
   and `gateway` (with a generated service token for the native admission
   listener), waits until each answers, runs the seed plan if given, and
   prints **one JSON object** on stdout (also saved as `report.json`).

Output (default `portal-rs/e2e/.harness/run`, ignored by git, replaced on
each run; override with `PORTAL_E2E_OUT`):

```json
{"status":"ready","database":"portal_e2e_<hex>",
 "baseUrls":{"public":"http://127.0.0.1:18080","internal":"https://127.0.0.1:18543",
             "nativeAdmission":"http://127.0.0.1:18091","gateway":"http://127.0.0.1:18090"},
 "outputDirectory":"...","certificateDirectory":".../certs/clients","caFile":".../certs/ca.pem",
 "internalServerName":"localhost",
 "clients":{"native-evidence":{"san":"native-evidence.game.e2e.oteryn.test",
            "env":"GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY","cert":"...pem","key":"...key"}, "...": {}},
 "witnessDirectory":"...","logs":{"migrate":"...","serve":"...","worker":"...","gateway":"..."},
 "seed":{"<step name>":{"...":"command output"}},"pending":[{"step":"channel","error":"pending","reason":"..."}],
 "pid":12345}
```

Client identities: `native-evidence`, `runtime-status`, `scope-assignment`,
`bootstrap-intents`, `lcfa`, `character-commands`, `character-services`,
`restore-notice`, `store`, `premium`, `loyalty-snapshot`, `loyalty-stream`,
`coins`, `sanctions`, `security-revocations`, `public-projections`. Runtime
status and scope assignment are bound to the scopes in
`PORTAL_E2E_NATIVE_SCOPES` (JSON array of `"<world UUIDv7>/<channel UUIDv7>"`,
default one `Entry` scope). Ports come from `PORTAL_E2E_HTTP_PORT`,
`PORTAL_E2E_INTERNAL_PORT`, `PORTAL_E2E_ADMISSION_PORT` and
`PORTAL_E2E_GATEWAY_PORT`. `PORTAL_E2E_BINARY` skips the build.
`--exit-after-ready` stops after the report. Ctrl-C or SIGTERM stops the
services and drops the database (`PORTAL_E2E_KEEP_DATABASE=1` keeps it).
The private CA key, CSRs, server key, gateway token and the full child
environment (with the database URL) stay in `<out>/secrets`, mode 0600.

### Seed commands

`oteryn-portal seed <entity> '<json object>'` runs only with
`OTERYN_ENV=testing` and `OTERYN_DATABASE_URL`, prints one JSON object on
stdout and exits 0 (done), 2 (`bad_input` / `conflict`), 3 (`pending` /
`not_applicable`) or 1 (`unavailable`). Every command is an idempotent
ensure: the same input gives the same output and no new row; other input for
an existing key is a `conflict`. A seed plan is `{"steps":[{"name","entity","args"}]}`;
`"${name.field}"` in later arguments is replaced by that step's output.

| entity | arguments | output |
|---|---|---|
| `account` | `email` (`*@example.invalid`), `password` (hashed with the portal's Argon2id), optional `accountId` (UUIDv7, generated otherwise), `locale` | `accountId`, `email`, `status` (`active`, e-mail verified), `nativeSecurityGeneration` (1) |
| `world` | `worldId` (UUIDv7), `name`, `region` (default `local`); `slug`, `host`, `port` are accepted but not stored | `worldId`, `name`, `region`, `notStored` |
| `oauth-native-client` | optional `clientId` (default `019a0000-0000-7000-8000-0000000c11e7`) | `clientId`, `enabled`, `firstParty`, `publicClient`, `nativeTicket` |
| `signing-trust` | `purpose`, `keyId`, `publicKeyHex` (Ed25519, 64 hex); `issuer`/`profile` default to the fresh-admission scope, or `urn:oteryn:platform:game-recovery` with purpose `existing_actor_recovery` | `issuer`, `profile`, `purpose`, `keyId`, `publicKey` (base64url), `issuerRevision` |
| `bootstrap-intent` | `accountId`, `operationId`, `targetWorldId`, `requestedName`, `requestedSex` (`Female`/`Male`), `profileRevision`, `rulesetRevision`, `contentRevision`, `starterTemplateRevision`, optional `ttlSeconds` (1..=300, default 300) | `operationId`, `issuerDecisionId`, `sourceRevision`, `accountId`, `targetWorldId`, `issuedAtSource`, `expiresAtSource`, `intentSha256` |
| `channel`, `route` | - | `pending`: the T19c `game_channels` registry and route record (gateway contract §7.3, mode 34a) are not on this tree |
| `native-evidence` | - | `not_applicable`: evidence is observed live; the first observation bootstraps the retained witness |

`signing-trust` writes the retained witness in
`GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`, so run it with the harness
environment (`<out>/secrets/env.json`). A bootstrap intent expires
`ttlSeconds` after issue, as the read contract bounds it.

## Native world source and expected refusals

`local.mjs` generates the CATALOG_EXPORT_V1 negative export in scratch using the
exact reference producer from Game
`d34807f5bd234ee8318e57ba454ba39be0df113d`, with its byte sidecar and explicit
provenance fence. The fixture's internal source revision is `a2a5da955dd8f580c9e768c8ac6a741db388cb22`;
that is distinct from the producer evidence pin. See
[`SOURCE.md`](../crates/world/tests/fixtures/catalog-669d5af5/SOURCE.md).
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
| `/{pl,en}/wiki`, `/wiki/search`, entity lists | 503 + no-store + shared shell | All eight export capabilities are `unsupported/unknown`, with no entities: [input fixture](../crates/world/tests/fixtures/catalog-669d5af5/unsupported-native-input.json#L9). Merged Game `main` explicitly refuses promotion of native capability payloads: [contract notes:54–78](https://github.com/Oteryn/Oteryn-Game/blob/669d5af5261a60d347167b7c880a4498aebe72ed/docs/reference/CATALOG-CONTRACT-NOTES.md#L54). There are no approved catalogue names/stats/relations to render. |
| `/{lang}/atlas` | 503 + no-store + shared shell | Needs a **Game-side atlas export**. At Game `d34807f5`, [`content/world/assets/catalog.json:1`](https://github.com/Oteryn/Oteryn-Game/blob/d34807f5bd234ee8318e57ba454ba39be0df113d/content/world/assets/catalog.json#L1) contains `assets:[]`; CATALOG_EXPORT_V1 has no map capability. No terrain/B3 substitute is rendered. |
| `/{lang}/atlas/delivery` | 503 + no-store + shared shell | Item/creature/loot/quest capabilities remain unsupported; no public active catalogue or accepted payloads. |
| `/{lang}/today/data`, `/today/widget` | 503 + no-store (JSON / HTML fragment) | G-12 is [PROPOSED, changes needed](../../docs/contracts/OTERYN_V2_WORLD_DAILY_FAMILY_CONTRACT_PROPOSAL.md#L1), with no accepted Game producer. No ingest evidence or supported catalogue names are invented. |
| `/{lang}/stats/world` without query | 200 + no-store + shared shell when registered worlds exist | Real registry picker; empty/unavailable registry still returns 503. |
| `/{lang}/stats/world?world=…&start=…&end=…` | 503 + no-store | Existing privacy/visibility policy is unavailable; the picker grants no metrics access. |

G-2 is resolved at Game `main` `669d5af5261a60d347167b7c880a4498aebe72ed`
(merged #3025): hidden identities never enter the export; an explicit `visibility` member
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

The focused paid top-up flow is `test specs/wallet-topup.spec.mjs`. The local
launcher configures a synthetic 1,234-coin / 250 PLN-minor-unit package and an
ephemeral test-provider HMAC key. Only `OTERYN_ENV=testing` plus
`OTERYN_SHOP_TEST_PAYMENTS_ENABLED=true` plus that explicit key registers
`/__test/payments/webhook`. It verifies the signature before parsing, and the
payments switch must permit processing. Production has no such route. The spec
creates the checkout through the CSRF-protected browser form, sends the signed
test webhook, waits for the real worker and reads the credited browser balance;
webhook and outbox replays must leave exactly one classified credit.

## Full shared-template matrix

The original flow specs remain. `inventory.py` scans module route declarations,
expands locale/prefix aliases and records module-owned navigation in startup order.
`inventory.json` retains sources, flags and methods. `route-expectations.json`
records the explicit actor, status and refusal reason for each executed route.
`routes.spec.mjs` exercises every inventory row; `navigation.spec.mjs` checks all
48 registry destinations, exact group links/labels, current section and keyboard
and touch activation. `content.spec.mjs` additionally publishes bilingual news,
events and Wiki articles and exercises media upload/read/delete. `oauth.spec.mjs`
proves the browser consent/PKCE/refresh path. Staff storage states stay in the
private per-run auth directory and are created by the existing admin flow.

Every full HTML destination receives the same framed shell, single h1, working
skip link, focus, overflow, local-link, form-CSRF and serious/critical axe checks.
Fragments, images, CSS and native API envelopes retain their protocol assertions;
they are not treated as standalone HTML pages. Expected HTTP failures in negative
flows are recorded separately. Script errors, unexpected console errors and
server errors other than explicitly qualified 503 refusals fail the suite.

After adding or changing a router, from `portal-rs/`:

```sh
python3 e2e/inventory.py
```

Review its provisional inventory and update the explicit route expectations.
Run the complete existing `node run.mjs ... test` command in all four projects.
Do not select one project for acceptance. After that run, from `portal-rs/e2e/`:

```sh
node summarize.mjs
```

The summarizer writes sanitized qualification.json and COVERAGE.md and fails if
any route or registry section lacks a passing case in any project, any case was
skipped/interrupted, any test failed, or any retry occurred. REPORT.md records
Rust acceptance and the concrete source fixes. The qualification source digest
excludes only REPORT.md, qualification.json and COVERAGE.md so that a subsequent
evidence-only commit can refer to the immutable tested source candidate.
