# CI, merge conditions and conflicts

Merging, labelling, re-running and pushing stay with whoever the owner
authorised; this file only says how the flow behaves.

## The lanes

| workflow | jobs | runs on |
|---|---|---|
| `portal-rs-ci.yml` (`Portal RS CI`) | `check`: `cargo fmt --all --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked` | PR and push to `main` touching `portal-rs/**` or the workflow; **not** in the Merge Queue; no database |
| `ci.yml` (`CI`) | `classify-changes`, `runtime-tests`, `platform-postgres (17)`, `platform-postgres (16)`, `test`, aggregator `platform-gate`; `php-coverage-report` on push and queue only | PR, Merge Queue (`merge_group`), push to `main` |
| retired heavy lanes (stubs) | `phase7-production-like-validation.yml` (incl. `premium-physical-e2e`), `edge-security-emulation.yml`, `platform-db-outage-validation.yml`, `game-auth-ticket-concurrency.yml` | PR only, when `classify-changes` selects them |
| `agent-governance.yml` | `checkpoint-validation` | PR and push touching `AGENTS.md`, `docs/agents/**`, `tools/agents/**` |

`runtime-tests`, `platform-postgres`, `php-coverage-report` and the heavy
lanes are stubs since the Laravel app was removed (2026-11-11): they keep
their check names and only print that the validation is retired. A portal-rs
PR starts them because the classifier treats `portal-rs/**` as `shared`.
They still have to pass. The required-check set
lives in the branch ruleset; read it before deciding a check does not
matter, and do not treat an unrequired portal-rs check as optional.

`platform-gate` is an aggregator: when it is red, open the failing leaf job
(`runtime-tests`, `platform-postgres (..)`, `test`), not the gate.

## Blind spot

The Merge Queue runs only `CI`. `Portal RS CI` judges each PR head alone,
so two portal-rs PRs that are green separately can break `main` together:
two migrations with one version, two edits to the `web` router or the
workspace `Cargo.toml`, a `Cargo.lock` that no longer matches. The first
signal is the `Portal RS CI` push run on `main`. Before a portal-rs PR goes
into the queue behind another one, build the merged tree locally
(`git merge-tree --write-tree origin/main HEAD`, then the checks in
`local-checks.md` on the merge) when both touch shared files or migrations.
PostgreSQL tests never run in CI (`local-checks.md`, *PostgreSQL tests*).

## Merge conditions

Start every merge round with the state of `main`:

```sh
.claude/skills/oteryn-platform-dev/scripts/main-health.sh portal-rs-ci.yml 10
.claude/skills/oteryn-platform-dev/scripts/main-health.sh ci.yml 10
```

If the latest push run on `main` is red, the next merge is the fix for it;
other merges wait.

A PR is ready for the queue when all of these hold on its **current** head:

1. Every check the PR started is green on that head (`Portal RS CI / check`
   for portal-rs paths, `platform-gate` and its leaves, any heavy lane the
   classifier selected), and the PR is not conflicted.
2. The template is complete: validation table rows for the exact head,
   compatibility and rollback for Game-facing changes, and the completion
   claim checked honestly (PostgreSQL tests run locally or marked not run).
3. Review threads are resolved or answered (`oteryn-platform-review`).

Merge through the Merge Queue with squash. After it lands, watch the push
runs on the new `main` (`Portal RS CI` and `CI`), then do the post-merge
closeout in the template: archive the task record, release ownership.

## Reading a red check

1. Confirm the run belongs to the current head (`head_sha` of the check run).
2. Open the leaf job log and search it for `error[`, `FAILED`, `panicked`,
   `test result: FAILED`; for PHP lanes, the uploaded JUnit artifact or the
   `pgsql-failing-tests.txt` artifact.
3. Ask whether `main` has the same failure (`main-health.sh`). If yes, it is
   not this PR's. Land one main fix first, then merge `origin/main` into the
   waiting PRs; say so once on each PR. Before writing a main fix, search
   open PRs for one that already exists.
4. Otherwise it is this PR's failure. Common causes:
   - `main` moved under the PR: merge `origin/main`, rebuild, reproduce.
   - Clippy on code that passed locally: you ran it per crate without
     `--all-targets`, or not on the merged tree.
   - Fixture bytes differ from the contract (`contracts.rs`, gateway Go
     vectors): the code drifted, not the fixture.
   - Duplicate migration version after a merge (`migrations.rs`).
5. Re-run a job at most once, and only when the log shows an infrastructure
   death (checkout, toolchain install, runner lost). A second failure is
   real; "flake" is never a root cause (`docs/agents/CONTEXT_ROUTING.md`,
   route `ci-repair`: root cause before rerun).

## Merge conflicts

```sh
git merge --no-edit origin/main
```

- Shared files are add-only (`portal-rs/README.md`): keep both added lines
  (router `.merge(...)` lines in `crates/web`, workspace members and
  dependencies), in the file's order.
- `portal-rs/Cargo.lock`: take `main`'s version
  (`git checkout origin/main -- portal-rs/Cargo.lock`), then run
  `cargo metadata --format-version 1 > /dev/null` in `portal-rs/` so cargo
  adds back only your dependencies; `--locked` builds must pass afterwards.
- Two migrations with one version: the later PR renames its own file to a
  new UTC timestamp; a merged migration is never renamed.
- Bring `main` in with a merge commit; only a real edit-versus-edit
  conflict in hand-written logic stops for a person.

## Before every push

- Run the checks in `local-checks.md` on the exact tree you will push.
- Push once, when they pass; collect follow-up fixes into the next push. A
  new push cancels the run in progress and starts every selected heavy lane
  again.
- Merge `origin/main` only when the PR conflicts, is red because `main`
  moved, or right before it enters the queue behind a related PR.
- Compile and test locally; CI confirms, it does not explore.

## Many PRs at once

- Plan tasks with disjoint paths can be open together (plan §10.1); land
  them one at a time through the queue and re-check every open portal-rs PR
  for a new conflict after each merge (`git merge-tree`).
- Every red or conflicted PR you own gets either a pushed fix or one comment
  naming the failing check and what blocks it.
