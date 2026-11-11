---
name: oteryn-platform-coding
description: Writing Rust, SQL and test code in Oteryn-Platform's portal-rs - scope discipline per plan task, a red-then-green check per task, Rust rules beyond the workspace lints, module boundaries, security and money rules, PostgreSQL queries and sqlx migrations, Game-contract fixtures and tests that wait instead of sleeping. Load before writing or changing code here, including delegated work.
---

# Writing code in portal-rs

Task layout and allowed paths are in `oteryn-platform-dev` and plan §10
(`docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`); this skill is how to write
the code itself. The Laravel app was removed; nothing is ported from it
(`oteryn-platform-dev`, *What the product is*).

## Scope: surgical and minimal

- Every changed line traces to the task. Adjacent code, comments and
  formatting stay as they are; new code follows the surrounding style.
  Unrelated dead code is reported in the PR and left in place; code your own
  change orphaned is removed.
- Build what the task row asks and nothing beside it: no extra feature,
  option, abstraction or error branch. Shrink a long draft before pushing.
- Stay inside the task's crate, its migration, `templates/<module>/` and one
  router line in `crates/web`. Shared files change only by adding lines.
- When the task reads two ways, pick the reading that fits the plan section
  and the contract, state it in one line in the PR or report, and carry on.
  Stop only for a choice that cannot be undone.

## Every task has a check that goes red, then green

Before production code, write or name the check that proves the task:

- a fix: a test that reproduces the bug on the current tree;
- a feature: tests for the success path, the refusal (unauthorized,
  flag off, limit reached) and bad input (plan §10.1), driving the real
  handler or repository, not only a pure function;
- a Game-facing route: a test that compares bytes with the fixture in
  `docs/contracts/fixtures/`;
- a refactor: the tests that pass before and after, unchanged.

Run it red first and keep the output. A PostgreSQL test that printed
`SKIPPED` proved nothing (`oteryn-platform-verify`). For several steps,
write `1. <step> -> check: <command>` so you can loop without asking.

## Rust in this workspace

The workspace lints (`portal-rs/Cargo.toml`) forbid `unsafe` and deny
`unwrap`, `expect`, `panic!`, `todo!` and `dbg!`. Errors are typed with
`thiserror` and propagate with `?`.

- Test files that need `unwrap` carry a file-level
  `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, as
  the existing ones do; production code never does. Any other new
  `#[allow]` sits on the narrowest item with a comment saying why.
- Modules depend only on `core`, `db` and the named interfaces
  (`identity::AccountId`, `wallet::Ledger`); `payments` and `bazaar` depend
  on `wallet`, never the reverse (plan §3.2). Domain types and rules are
  free of I/O; the repository (sqlx) and handlers sit beside them in the
  crate.
- Configuration comes from `OTERYN_*` / `PORTAL_*` variables read through
  the existing config types; secrets and the database URL never reach a log
  or a `Debug` output (`crates/core` redacts the URL; keep it that way).
- Borrow before cloning: `&str` / `&[T]` parameters, no `.clone()` in loops,
  no `.collect()` that is only iterated again.
- A `TODO` names its plan task or Issue.

## Web, security and money

- Every mutation is authorized server-side; browser mutations go through
  the CSRF and session layers in `crates/web` (`csrf.rs`, `session.rs`) and
  the shared rate limiter (`ratelimit.rs`) where abuse is possible.
- Templates are askama; never build HTML from strings. Markdown goes
  through the sanitizer the plan names (pulldown-cmark and ammonia).
- Money and scarce goods (wallet, store, bazaar, payments): one transaction,
  the rows locked (`FOR UPDATE`) in a stable key order before mutating
  dependents, and an idempotency key so a replay returns the first result.
  `crates/wallet/src/ledger.rs` is the pattern.
- Anything that touches Game or money sits behind a flag, off by default.
- Game boundary: idempotent, keyed by an operation id, written through the
  outbox (`crates/db/src/outbox.rs`), reconciled; a timeout is unknown, not
  failure. Wire output is canonical and byte-exact
  (`crates/internal-api`); compare secrets in constant time.
- Pages work at 360 px width and meet WCAG 2.2 AA (plan §10.1).

## PostgreSQL and migrations

- A migration is one new file `portal-rs/migrations/<UTC timestamp>_<task>_<what>.sql`
  with a version no other file uses; it is embedded by `sqlx::migrate!` in
  `crates/db`. Never edit or rename a merged one; fix forward.
- Migrations are additive: new tables, columns with defaults, new indexes.
  Never assume production data is empty.
- A foreign-key column that is not the leading column of an existing index
  gets its own index.
- Keep transactions short: no network calls, waits or rendering inside.
- Queues take rows with `FOR UPDATE SKIP LOCKED`; a cap (slots, bids,
  balances) is checked in the same statement or transaction that writes.
- Every read over accounts, characters or history has a `LIMIT`, a page or
  a proven small bound, and matches an index.
- Compare nullable values with `IS DISTINCT FROM`; aggregates over an empty
  set return `NULL`, so handle it.
- Bind parameters; no SQL built from user input. A dynamic identifier (a
  test schema name) is generated, never taken from input.

## Tests

- Unit tests sit beside the code (`*_tests.rs`, `#[cfg(test)]`);
  integration and contract tests in `crates/<crate>/tests/`.
- PostgreSQL tests read `PORTAL_TEST_DATABASE_URL`, create their own schema
  (`crates/bazaar/tests/support/mod.rs`) and drop it; they never share rows
  with another test.
- Tests wait for the condition they care about, never for a guessed
  duration: a channel, `Notify` or readiness handle, or a poll with a
  deadline; drive timers with `tokio::time::pause` / `start_paused`.
- Fixtures under `docs/contracts/fixtures/` are read, never rewritten to
  match the code. Assertions stay as strict as before.

## Sources

Adapted, not copied verbatim: the Oteryn-Game `oteryn-coding` skill;
Andrej Karpathy's coding guidelines as written up in
forrestchang/andrej-karpathy-skills (MIT); apollographql/skills
`rust-best-practices` (MIT); supabase/agent-skills
`supabase-postgres-best-practices` (MIT); `portal-rs/README.md` and plan
§3, §6, §10.
