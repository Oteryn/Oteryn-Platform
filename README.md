# Oteryn Platform

Oteryn Platform is the first-party web/application platform for the Oteryn Open Tibia Server ecosystem. It is intended to replace MyAAC as the long-term web platform while the Oteryn game server remains a separate project owned by Oteryn-Game.

## Current implementation

The platform is the Rust portal in `portal-rs/`: one binary (`oteryn-portal`, roles `serve`, `worker`, `migrate`, `gateway`) over its own PostgreSQL 18.6, built task by task from `docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`. Its commands and rules are in `portal-rs/README.md`.

`legacy/` holds the earlier Laravel 13 application and the Go game gateway (`legacy/services/game-gateway/`), moved there on owner decision 2026-11-11. That tree is frozen: no workflow builds or tests it, it is not extended, and it stays only as a reference and as the source of the one-time legacy data export until cutover (plan §9). See `legacy/README.md`.

`AVAILABLE` means at least one validated repository capability exists; it does not imply complete product scope, production proof or activation authority. Read `docs/architecture/MODULE_CATALOG.md` and exact task/PR evidence before making a completeness claim.

## Local setup

Requirements: the Rust toolchain pinned in `portal-rs/rust-toolchain.toml` and, for the database-backed roles and tests, PostgreSQL 18.6.

```sh
cd portal-rs
cargo build
OTERYN_DATABASE_URL=postgres://... cargo run -p oteryn-portal -- migrate
OTERYN_DATABASE_URL=postgres://... cargo run -p oteryn-portal -- serve
```

## Game server and integration boundaries

Game-server behavior is an external contract documented in the Oteryn-Game docs. The portal shares no database with the game server: Game reaches the portal only through `/internal/v1` (mTLS), the login gateway role calls the native-admission route with a service token, and world data is read from a pinned Game release. The contracts and their shared fixtures are in `docs/contracts/`; every mutation across the boundary is idempotent and keyed by an operation id.

Configuration comes from `OTERYN_*` / `PORTAL_*` environment variables. Do not reuse game-server credentials or database root/admin accounts.

## Validation

```sh
cd portal-rs
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
PORTAL_TEST_DATABASE_URL=postgres://... cargo test --locked
```

PostgreSQL-backed tests skip without `PORTAL_TEST_DATABASE_URL`; a skipped test is not evidence. GitHub Actions runs these checks in Portal RS CI, plus the repository governance validators in CI. Required evidence remains tied to the exact tested head.

## Licensing and contributions

Oteryn Platform is proprietary. Public repository visibility does not make the project open source and does not grant permission to copy, modify, redistribute, sublicense, host or commercially use the project or a derivative of it.

`LICENSE.md` is the canonical proprietary notice. It applies only to original Oteryn Platform material that does not carry a separate notice. Third-party dependencies, assets, game data, fixtures and protocol or compatibility material remain governed by their own rights and are bounded by `THIRD_PARTY_NOTICES.md`.

External contributions are not accepted by default. Do not submit code, documentation, assets or data unless the repository owner has invited the contribution and agreed written contribution terms before acceptance. See `CONTRIBUTING.md`.

A future source-available, open-source, dual-license or component-specific policy requires a separate owner-approved decision and a provenance review of the exact material covered.

## Authoritative project documentation

- `AGENTS.md` and `docs/agents/PLATFORM_AGENT_BOOTSTRAP.md` — mandatory operating rules for agents.
- `LICENSE.md` — canonical repository licensing and use boundary.
- `THIRD_PARTY_NOTICES.md` — third-party rights and unresolved provenance boundary.
- `docs/agents/PROJECT_STATE.md` — current project phase and next work.
- `docs/agents/REPOSITORY_MAP.md` — repository navigation and ownership map.
- `docs/architecture/ARCHITECTURE_AUTHORITY.md` — architecture precedence and canonical routing.
- `docs/architecture/SYSTEM_ARCHITECTURE.md` — system boundaries and target topology.
- `docs/architecture/MODULE_CATALOG.md` — module ownership and repository-availability classification.
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md` — current portal assessment, remaining decisions and completion gate.
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md` — accepted player-tools architecture and delivery priorities.
- `docs/architecture/SECURITY_ARCHITECTURE.md` — mandatory security invariants.
- `docs/architecture/DATA_OWNERSHIP.md` — persistent-data ownership rules.
- `docs/contracts/` — Platform/Game integration and operation-specific contracts.
- `docs/agents/tasks/active/` — active implementation/task records.

Repository state, accepted ADRs, focused architecture, task records, Git and live PR/CI state are authoritative over chat history.