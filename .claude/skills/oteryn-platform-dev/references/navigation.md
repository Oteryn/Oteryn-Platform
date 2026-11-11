# Finding code without reading it all

`portal-rs/` is a Cargo workspace: `bin/oteryn-portal` plus `crates/*`
(package names `oteryn-<crate>`, e.g. `oteryn-db`, `oteryn-gameauth`,
`oteryn-internal-api`). The rest of the tree is the legacy Laravel app, the
Go gateway, deploy tooling and docs. The plan
(`docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`, about 900 lines, Polish) and
the contracts are long: read the section you need, never the whole file.

## How to read

1. Locate first: `rg -l '<symbol>' portal-rs/crates`, then
   `rg -n '<symbol>' <file>` for the line numbers.
2. Read the range with the Read tool (`offset`, `limit`), about 60-150 lines.
3. Plan sections by heading: `rg -n '^#' docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`;
   a task row: `rg -n '^\| T52 ' docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`.
4. When the session has rust-analyzer LSP tools, use go-to-definition and
   find-references instead of grepping for call sites.
5. A question that needs many files goes to one `Explore` subagent or
   `oteryn-platform-ref-reader`; their contents stay out of this context.
6. A file already read in this session is not read again unless it changed.
7. Build and test output goes to a file; read its tail and the `error[`,
   `FAILED`, `panicked` and `SKIPPED` lines.
8. Documents are task-routed (`docs/agents/CONTEXT_ROUTING.md`); do not
   follow links recursively. Task records and evidence under `docs/agents/`
   are history, not current authority.

## Where things live

| looking for | start here |
|---|---|
| a module's design and task | plan §3.2 (crate layout), §5 (HTTP surfaces), §10 (tasks with allowed paths) |
| crate list and dependencies | `portal-rs/Cargo.toml` (`members`, `[workspace.dependencies]`, `[workspace.lints]`); `cargo metadata --no-deps --format-version 1` from `portal-rs/` |
| binary roles and startup | `portal-rs/bin/oteryn-portal/src/` (`main.rs`, `worker.rs`) |
| web router, sessions, CSRF, rate limits | `crates/web/src/` (`lib.rs`, `surface.rs`, `session.rs`, `csrf.rs`, `ratelimit.rs`) |
| migrations and the migrator | `portal-rs/migrations/*.sql`; `MIGRATOR` in `crates/db/src/lib.rs`; outbox in `crates/db/src/outbox.rs` |
| game-facing `/internal/v1` | `crates/internal-api` (guards, canonical JSON, fixtures loader); contract tests in `crates/internal-api/tests/contracts.rs` |
| login gateway (role `gateway`) | `crates/gateway` and its `README.md`; legacy Go source it ports: `legacy/services/game-gateway/internal/` |
| PostgreSQL tests | `rg -l PORTAL_TEST_DATABASE_URL portal-rs` (e.g. `crates/bazaar/tests/postgres.rs`, `crates/wallet/tests/postgres.rs`, harness in `crates/bazaar/tests/support/mod.rs`) |
| templates and static files | `portal-rs/templates/` (askama), `portal-rs/static/` |
| a contract and its fixtures | `docs/contracts/*.md`, `docs/contracts/fixtures/<name>/` |
| which CI lanes a path starts | `python3 scripts/ci/classify_changes.py --paths <path...> --json` |
| rules for a directory | `portal-rs/README.md`, a crate's `README.md` (e.g. `crates/gateway`, `crates/bazaar`), the nearest `AGENTS.md` |
| active work and owners | live Issues and PRs first; then `docs/agents/tasks/active/` |
| what Laravel did (needs only) | plan §5.5 table *Laravel dziś*; read legacy code only to learn a need or a contract, never to port it |
