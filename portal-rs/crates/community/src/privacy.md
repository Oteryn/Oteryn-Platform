# T45 privacy delivery

Branch: `codex/portal-rs-t45-privacy`; base: `origin/main` at `3d6a1ae2`.
Owner requested a branch push only: no PR, workflow changes, merge or deployment.

Platform changes: owner-only hidden/hide-all, one main character, escaped 500-character
comment, seven plan switches and D11 online-time preference. New profiles default
hidden with every switch off; existing guild values and unbound legacy rows survive.
Account status/association preferences remain upper bounds. The owner transaction
serializes changes, advances privacy revision and writes a bounded audit event;
30 successful writes/account/minute is a shared database budget. Session/CSRF
protection is unchanged and exercised in the browser.

Community configured routers use current T45/T09c adapters. Online is live with
accepted Game presence/profile pushes and current unique LCFA World/name ownership.
Unknown, unsaved, hidden, status-off, ambiguous and unavailable state denies.
Game profile/presence epoch fences prevent mixed-generation or suppressed data;
a newer profile cannot authorize older presence, and visibility is rechecked after
reading. T24 stores normalized bodies: its existing guarded ingest is the Game
source boundary, not an invented envelope field on the stored payload.

Statistics use async serve-time privacy checks, without blocking runtime threads
or caching allows. Only the two necessary privacy calls in stats/world were changed;
its routing is untouched. Guild source and world policy retain independent Game
acceptance gates. `OnlineOwner::configured` narrows a qualified ownership-history
provider; LCFA cannot invent `owned_since` or retained-set coverage. No online-time
route was added. Public character duration remains forbidden/unavailable.

Historical killer labels are withheld. G-4 supplies historical names without stable
killer IDs/current floors; names are never mapped to a current holder. Death
permission independently checks the victim's current hidden/deaths preferences.

## Remaining Game blockers and 503s

Coordinator update: Game main `d34807f5bd234ee8318e57ba454ba39be0df113d`,
`OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` revision 2 (2026-11-07). This update is the
activation authority; repository proposals and synthetic tests do not prove a
Game producer. Game repositories were not accessed.

| Surface (also PL/EN) | Exact blocker and local evidence |
| --- | --- |
| `/deaths`, `/characters/{name}/deaths` | G-4 character_deaths is CANDIDATE rev 1, without a producer. Explicit configured contract gate: `crates/community/src/deaths.rs:271`; real privacy is wired. |
| `/guilds`, `/guilds/{name}` | G-4 guild is CANDIDATE rev 1, without a producer. Gate: `crates/community/src/guilds.rs:221`. Contract refusal is now HTTP 503, replacing the old HTTP 200 unavailable document. |
| `/houses`, house details | G-4 house is CANDIDATE rev 1, without a producer. Gate: `crates/community/src/houses.rs:160`; no owner identity is inferred from names. |
| `/stats/guilds`, `/stats/world` | G-8 statistics has no accepted contract/producer. Independent gate: `crates/stats/src/privacy.rs:53`; guild entry point: `crates/stats/src/guilds.rs:62`; world wiring: `crates/stats/src/world/http.rs:22`. Flags cannot bypass it. |
| Online-duration services | No accepted G-8 public duration exposure, ownership-history or retained-set coverage source. No new route; existing refusal: `crates/stats/src/rankings_online.rs:161`. The wrapper supplies privacy only, without fabricating the missing Game evidence. |
| `/wiki/places` | No accepted G-9 journal/share producer/contract or qualified T81 public card feed. `crates/wiki/src/places/README.md:13` and `:20`; unchanged. |

Cold `/online` may also return 503 when accepted push data, LCFA or current privacy
state is missing/stale/unreadable. This is a dependency refusal, not an unaccepted
Game contract gate. With valid evidence and explicit owner opt-in it returns 200,
with only visible characters counted. There is no production seeding or mock.

## T81 / wiki places: report only

No wiki/places code was changed. Its existing owner-only `PlacesStore::daily(actor)`
is not a public card feed. T81 needs an approved bounded public feed, public place
and editorial bindings, purpose and party-size cohorts (1/2/4), and qualified kill
rate evidence for estimates. Missing metrics must remain absent; no fabricated
zeroes, party sizes, encounter availability or boss spawn claims.

The G-9 proposal's section 8 (`docs/contracts/OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md:238`)
proposes place/level-band/vocation cohorts with at least 10 complete sessions and
5 accounts, delay at least 24h, account-exclusion recomputation/fences and protection
against small cells/differencing. It explicitly does not authorize party data.
These are proposals requiring separate acceptance; a party-size public feed cannot
be inferred from them or from private session rows.

Section 9 (`...CONTRACT_PROPOSAL.md:242`) keeps `playerdata.hunts.share` false.
G-9 must specify owner consent, revocation, expiry, moderation and account erasure
before journal sharing can publish anything. Moderator approval alone is insufficient.
These missing authority/product decisions leave no additional fully specified
Platform-owned implementation to add in this task.

## Verification

PostgreSQL 18.6, max_locks_per_transaction=1024, max_connections=300. Tests use
throwaway local databases and isolated schemas, with `PORTAL_TEST_DATABASE_URL`.
No production credentials, personal data or browser traces are committed.

Material implementation validated: `bc4179837459adb7a52630c0c04f3c0b4edfd87d`. The subsequent evidence commit
changes only this report and the task checkpoint.

| Command / check | Exact result |
| --- | --- |
| `cargo fmt --all --check` | PASS, exit 0. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | PASS, exit 0; no warnings. |
| `cargo test --locked --no-fail-fast` | PASS, exit 0; 1,101 passed, 0 failed, 3 ignored across 83 test/doc-test targets. Fresh migrated PostgreSQL 18.6; `RUST_TEST_THREADS=4`; requested 1024 locks / 300 connections. |
| `CHROMIUM_PATH=/usr/bin/chromium node run.mjs /tmp/portal-t45-e2e-run3/env.json test privacy.spec.mjs` | PASS: 4 passed, 0 failed, 0 retries, 47.1s; PL/EN at 1440 and 390 pixels. Local harness stopped afterward. |
| `cargo sqlx migrate revert`, then `cargo sqlx migrate run` | PASS: 20261107130001 down/up round trip on an isolated database. The full chain also migrated successfully on the fresh final test database. |
| `node --check` for both changed E2E specs | PASS. |
| `git diff --check`; task checkpoint validation | PASS. |

The three pre-existing ignored Rust tests are the 360px premium/account browser
layout, 360px store browser page, and search keyboard/accessibility tests.
They require `PORTAL_BROWSER_MODULES` (and browser dependencies). The separate
Playwright privacy matrix above ran all four selected projects with Chromium.

The final workspace run includes the new owner/privacy/audit PostgreSQL test,
real-ingest/real-online-adapter regression, and real statistics history/ranking
privacy regression. Hidden and unknown characters are excluded, off switches
remove fields/events, missing/error state denies, and foreign mutations refuse.
Browser proof additionally covers real sessions, CSRF, account upper bounds,
comment escaping and all documented contract-gated 503s.

Development verification found and corrected the normalized-payload authority
assumption, stale killer-label test expectations, the browser CSRF selector's
scope, and guilds' legacy HTTP 200 refusal. The initial shared database also had
untracked pre-existing tables that collided with migrations; final acceptance
uses a fresh database with the complete migration chain applied first. A superseded
serial run was stopped after its obsolete privacy regression failure was fixed.

## Changed files

- `docs/agents/tasks/active/OTERYN-20261107-portal-rs-t45-privacy.md`
- `portal-rs/Cargo.lock`
- `portal-rs/crates/community/src/privacy.md`
- `portal-rs/crates/characters/src/profile/README.md`
- `portal-rs/crates/characters/src/profile/authority.rs`
- `portal-rs/crates/characters/src/profile/http.rs`
- `portal-rs/crates/characters/src/profile/mod.rs`
- `portal-rs/crates/characters/src/profile/tests.rs`
- `portal-rs/crates/community/Cargo.toml`
- `portal-rs/crates/community/src/deaths.rs`
- `portal-rs/crates/community/src/deaths/README.md`
- `portal-rs/crates/community/src/deaths/tests.rs`
- `portal-rs/crates/community/src/deaths/web.rs`
- `portal-rs/crates/community/src/guilds.md`
- `portal-rs/crates/community/src/guilds.rs`
- `portal-rs/crates/community/src/guilds_pages.rs`
- `portal-rs/crates/community/src/houses.rs`
- `portal-rs/crates/community/src/lib.rs`
- `portal-rs/crates/community/src/online.rs`
- `portal-rs/crates/community/src/privacy.rs`
- `portal-rs/crates/community/src/profiles.md`
- `portal-rs/crates/community/src/profiles.rs`
- `portal-rs/crates/community/tests/privacy_authority.rs`
- `portal-rs/crates/stats/Cargo.toml`
- `portal-rs/crates/stats/src/guilds.rs`
- `portal-rs/crates/stats/src/guilds/http.rs`
- `portal-rs/crates/stats/src/guilds/tests.rs`
- `portal-rs/crates/stats/src/lib.rs`
- `portal-rs/crates/stats/src/online/README.md`
- `portal-rs/crates/stats/src/privacy.rs`
- `portal-rs/crates/stats/src/store.rs`
- `portal-rs/crates/stats/src/world.rs`
- `portal-rs/crates/stats/src/world/http.rs`
- `portal-rs/crates/stats/tests/privacy.rs`
- `portal-rs/e2e/specs/privacy.spec.mjs`
- `portal-rs/e2e/specs/public.spec.mjs`
- `portal-rs/migrations/20261107130001_t45_privacy.down.sql`
- `portal-rs/migrations/20261107130001_t45_privacy.up.sql`
- `portal-rs/templates/profile/en.ftl`
- `portal-rs/templates/profile/pl.ftl`
- `portal-rs/templates/profile/settings.html`
