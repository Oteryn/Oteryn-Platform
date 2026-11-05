# Local checks before a push

The commands are copied from `.github/workflows/portal-rs-ci.yml` and
`portal-rs/README.md`, which win if they differ. Run what your paths need,
but run all of it, and fix every failure of one run in one push.

## Which lanes will run

```sh
python3 scripts/ci/classify_changes.py --base origin/main --head HEAD --json
```

Any `portal-rs/**` path is class `shared`: besides `Portal RS CI` it starts
the core `CI` workflow and every heavy legacy lane (`phase7`, `edge`,
`db_outage`, `game_auth_concurrency`). `docs/contracts/**` does the same.
`docs/**` and `docs/agents/**` start none.

## portal-rs (order: cheapest first)

All from `portal-rs/`; the toolchain is pinned in `rust-toolchain.toml`
(rustup installs it on first use).

1. If the branch conflicts with `main`, merge `origin/main` first
   (`ci-merge.md`, *Merge conflicts*); then `cargo fmt --all`.
2. `cargo fmt --all --check`.
3. While working, per changed package:
   `cargo clippy --locked -p oteryn-<crate> --all-targets --quiet -- -D warnings`
   and `cargo test --locked -p oteryn-<crate>` (narrow with
   `--test <target> <filter>`; add `--no-fail-fast` to see every failing
   target in one run).
4. Before the push, the CI commands on the whole workspace:
   `cargo clippy --locked --all-targets -- -D warnings` and
   `cargo test --locked`. A change to a shared crate (`core`, `db`, `web`,
   `internal-api`) or to the workspace `Cargo.toml` always needs the whole
   workspace.
5. Migrations added or renamed:
   `cargo test --locked -p oteryn-db --test migrations` (unique versions,
   embedded set), and a real `migrate` against a scratch PostgreSQL 18.6
   (*PostgreSQL tests* below).
6. Contract or fixture touched: `cargo test --locked -p oteryn-internal-api --test contracts`,
   and the crate that reads that fixture (`rg -l 'docs/contracts/fixtures' portal-rs/crates`).
7. `git diff --check` and a read of the full diff.

The test runner is `cargo test`; no nextest profile exists in this
repository, so use `cargo nextest run` only as a local convenience and
never cite it as the CI result.

## PostgreSQL tests

- Database tests read `PORTAL_TEST_DATABASE_URL` and print
  `SKIPPED ... PORTAL_TEST_DATABASE_URL is unset` and pass when it is
  missing. `portal-rs-ci.yml` has no database service, so **CI never runs
  them**: a green `Portal RS CI` proves nothing about SQL, migrations,
  locking or transactions.
- For any change to SQL, migrations or a transactional path, run them
  locally against PostgreSQL 18.6 (the plan's version, D24), e.g. a
  throwaway `postgres:18.6` container on localhost, and export the URL
  without printing it:
  `set +x; export PORTAL_TEST_DATABASE_URL=...; cargo test --locked -p oteryn-<crate>`.
  Then check the output has no `SKIPPED` line for the tests you rely on.
- Apply the migrations once from scratch:
  `OTERYN_DATABASE_URL=... cargo run -p oteryn-portal -- migrate`.
- Where no database exists (a worker whose packet forbids Docker), report
  the PostgreSQL tests as not run, with the reason; never as passed.
- Harnesses create their own schema per test
  (`crates/bazaar/tests/support/mod.rs`); a new PostgreSQL test does the same.

## Legacy lanes that still gate a PR

The core `CI` workflow runs on portal-rs PRs too. Its `runtime-tests` job
runs `composer audit`: a newly published advisory turns every PR red,
whatever it changes. That is a `main` failure, fixed once by a Composer
lock bump PR (`ci-merge.md`, *Reading a red check*), not in your PR. A
PHPStan finding at `app/Console/Commands/InstallWikiLaunchContent.php:49`
is known and pre-existing; report it as such.

## Agent governance

When `AGENTS.md`, `docs/agents/**` or `tools/agents/**` changed, run the
offline steps of `agent-governance.yml`:

```sh
for t in test_checkpoint test_source_branch_closeout test_task_liveness \
         test_task_issue_liveness test_documentation_ia test_control_room; do
  python3 tools/agents/$t.py || break; done
python3 tools/agents/documentation_ia.py
python3 tools/validation/test_prompt_eval.py && python3 tools/validation/prompt_eval.py
python3 tools/agents/checkpoint.py docs/agents/tasks/active/<task>.md --require-checkpoint
```

The policy-consistency and live-liveness steps need the bound META checkout
and a GitHub token; leave them to CI.

## Build hygiene

- One Rust build per target dir at a time; parallel builds block on the
  cargo lock and look like a hang. A worktree gets its own
  `CARGO_TARGET_DIR`, deleted after the merge.
- Check free disk before a cold build; "No space left on device" mid-build
  leaves a corrupt incremental cache: delete the target dir and rebuild.
- Take the exit code of the command, not of `tail`/`tee`:
  `set -o pipefail` or `${PIPESTATUS[0]}`.
