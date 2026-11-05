---
name: oteryn-platform-dev
description: Router for landing a change in Oteryn-Platform, whose product is the Rust portal in portal-rs/ - the path from governing Issue and task to merge, the rules that hold at every step, the cross-repository contract with Oteryn-Game (game auth tickets and login, catalog and world data, store and premium delivery), portal-rs migrations, local checks, CI lanes, the Merge Queue and which other oteryn-platform skill to load. Load first for any change to this repository, including a red Portal RS CI or platform-gate, a composer audit failure or a conflicted PR.
---

# Landing work in Oteryn-Platform

This skill records current repository practice. It grants no authority:
root `AGENTS.md` and `docs/agents/PLATFORM_AGENT_BOOTSTRAP.md` set what an
agent may do, and the META policy bound by
`docs/agents/META_AGENT_POLICY_BINDING.json` wins over this text. Workflows,
`portal-rs/README.md` and `docs/architecture/OTERYN_PORTAL_RUST_PLAN.md` win
over the commands here; if they disagree, fix this skill in the same change.

## What the product is

The new Platform is `portal-rs/`: one Rust binary (`oteryn-portal`, roles
`serve`, `worker`, `migrate`, `gateway`; the plan adds `admin-cli`) over its own
PostgreSQL 18.6, built task by task from the plan (owner decision D22,
2026-11-05; task table in §10). The Laravel tree (`app/`, `routes/`,
`database/`, `tests/`, `composer.*`) and the Go gateway
(`services/game-gateway/`) are legacy: the plan uses them only as a source
of needs and contracts, nothing is ported from them, and they are switched
off and removed by a separate PR at cutover (plan §1, §9). Do not extend
them; touch them only for a task that names them (security fix, dependency
advisory, contract evidence).

## The skills and when to load them

| skill | load when |
|---|---|
| `oteryn-platform-dev` (this) | any change: the path below, the rules, the Game contract |
| `oteryn-platform-coding` | before writing Rust, SQL or test code |
| `oteryn-platform-debugging` | something fails or misbehaves, before a fix |
| `oteryn-platform-review` | before marking ready; when review findings arrive |
| `oteryn-platform-verify` | before any claim: done, fixed, green, mergeable |
| `oteryn-platform-waves` | planning several plan tasks, delegating, landing several PRs |

The optional UI skills in `.agents/skills/` keep their own permission gate
(root `AGENTS.md`); these skills do not grant it.

## The path of one change

| Step | What | Read |
|---|---|---|
| 0 | Find the code: `rg`, line ranges, one Explore subagent for wide questions | `references/navigation.md` |
| 1 | Governing live Issue, plan task id and its allowed paths (§10), overlap search (open PRs, `docs/agents/tasks/active/`), nearest `AGENTS.md` | root `AGENTS.md`, plan §10 |
| 2 | Substantial work: active packet from `docs/agents/tasks/TASK_TEMPLATE.md` with `owned_paths`; one branch | `CONTRIBUTING.md` |
| 3 | Write the check that goes red, then the code; migration if the module needs one | `oteryn-platform-coding` |
| 4 | Game-facing change: contract, fixtures, flag | *Cross-repository contract with Oteryn-Game* below |
| 5 | Local checks for the changed paths, on the exact tree you will push | `references/local-checks.md` |
| 6 | Review your own diff; PR from `.github/PULL_REQUEST_TEMPLATE.md` with the validation table filled for the exact head | `oteryn-platform-review` |
| 7 | One push; CI lanes, red checks, conflicts, Merge Queue, closeout | `references/ci-merge.md` |

Read only the reference for the step you are on. Step 5 is never skipped.
A worker running from a packet (`.claude/agents/`) does steps 0-6 and the
push its packet allows, then reports; step 7 belongs to the coordinator.

## Rules that hold at every step

- Writes go only to `Oteryn/Oteryn-Platform`, on a task branch, never
  directly to `main`; no force-push, no bypassed protection, no weakened gate.
- Oteryn-Game and other server repositories are not read, searched or
  changed without the owner's explicit grant for that repository and task
  (`PLATFORM_AGENT_BOOTSTRAP.md`, *Platform-only default*). Platform proves
  its side of a contract from its own contracts, fixtures and tests.
- One plan task changes one module crate (`crates/<module>`), its own
  migration, `templates/<module>/` and one line where its router is merged in
  `crates/web` (`portal-rs/README.md`). Shared files (workspace
  `Cargo.toml`, the `web` router, `migrations/`) change only by adding lines.
- Migrations are additive and a merged one is history:
  `portal-rs/migrations/<UTC timestamp>_<task>_<what>.sql`, a version no
  other file uses (`crates/db/tests/migrations.rs` guards it, #1721).
- Every game- or money-touching feature is behind a flag, off by default.
- Tests stay as strict as they were: a red test is fixed, not skipped,
  `#[ignore]`d, loosened or deleted; no new `#[allow]` to silence a lint.
- `.github/workflows/**`, `scripts/ci/**` and `tests/ci/**` change only
  inside a task that owns them (plan §10.1 forbids workflows in plan tasks).
- `.env`, tokens, database URLs, service-token files and keys stay unread,
  unprinted and uncommitted; configuration is `OTERYN_*` / `PORTAL_*`
  environment variables (`set +x` before exporting a URL).
- Security-sensitive paths (identity, sessions, MFA, gameauth tickets and
  grants, admin/RBAC, wallet, store, payments, bazaar, internal API guards)
  get success, refusal and bad-input tests and a risk note in the PR.
- A green check counts for the head SHA it ran on, nothing else.
- Deploy, protected environments, production data and live payments need
  task-specific authority; merge authority is not production authority.
- Commits and PR titles: `<type>(portal-rs): <summary>`; squash merge.
- Claims need fresh evidence (`oteryn-platform-verify`); another agent's
  report is a claim.

## Cross-repository contract with Oteryn-Game

Platform must keep working with the game server. There is no shared
database; Game reaches the portal only through `/internal/v1` (mTLS, plan
§5.3, `crates/internal-api`), the login gateway calls
`/internal/v1/game-auth/native-admissions` with a service token, and the
portal calls nothing in Game (plan §5.3.2). World data is read-only from a
pinned Game release (plan §6.1, `crates/world`). The flow table with each
contract's state on the Game side is plan §6; check it before a task.

| surface | contract (`docs/contracts/`) | portal-rs | proof |
|---|---|---|---|
| login, game tickets, admission grants | `OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`, `GAME_GATEWAY_IDENTITY_CONTRACT.md`, `OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md` | `crates/gameauth`, `crates/gateway` (replaces `services/game-gateway` byte for byte, T52) | `crates/gateway/tests/` (Go vectors in `fixtures/go_vectors.jsonl`, `game_client_native_login_fixtures.rs`), `crates/gameauth/src/evidence_tests.rs` on `fixtures/oteryn-native-v1/` |
| internal routes and guards | the contract of each route (plan §5.3.1) | `crates/internal-api` | `crates/internal-api/tests/contracts.rs` reads `docs/contracts/fixtures/` |
| game catalog, world data | `OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md`, `docs/contracts/game-catalog/` | `crates/world`, `crates/wiki`, `crates/atlas` | crate tests; the Game pin is a deliberate change |
| store and premium delivery | `OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`, `OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md`, `OTERYN_V2_STORE_WIRE_CONTRACT.md`, `OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md`, `OTERYN_V2_LOYALTY_SNAPSHOT_CONTRACT.md` | `crates/store`, `crates/wallet` | `crates/store/src/premium_tests.rs` on `fixtures/premium-snapshot-v1/`; legacy lane `premium-physical-e2e` (`phase7-production-like-validation.yml`) runs Laravel against a pinned Game SHA |

- Routes Game already implements keep their contract byte for byte:
  members, types, result codes, digests, idempotency keys, size limits,
  503/401 bodies (plan §5.3). Fixtures in `docs/contracts/fixtures/` are
  shared truth; a fixture change is a contract change.
- A contract's status bounds the work: many are `PROPOSED` or `CANDIDATE`
  and authorize no implementation until accepted (plan §6 lists the ones the
  portal must not implement from yet).
- Every mutation across the Game boundary is idempotent, keyed by an
  operation id, goes through the outbox (`crates/db`) and has
  reconciliation; a timeout is neither success nor failure (plan §6).
- A feature without Game data shows "unavailable" with the last update
  time, never invented values.
- Game-side work is a separate task (G-1, G-2, ...) sent by the coordinator;
  name it in the PR's *Compatibility, rollout, and rollback* section.

## Scripts

- `scripts/main-health.sh [workflow] [runs]` - last push runs of a workflow
  on `main` (default `portal-rs-ci.yml`) with their failing jobs; run it
  before a merge round.
