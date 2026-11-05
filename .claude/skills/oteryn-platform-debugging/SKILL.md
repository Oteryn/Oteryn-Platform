---
name: oteryn-platform-debugging
description: Root-causing a failure in Oteryn-Platform before any fix - a failing or flaky portal-rs test, a red Portal RS CI or platform-gate leaf job, a PostgreSQL test that only fails on a real database, a fixture byte mismatch against a Game contract, a red push run on main, or a composer audit advisory. Load when something fails or misbehaves, before writing the fix.
---

# Debugging in Oteryn-Platform

No fix without a root cause. A fix pushed on a guess costs a CI cycle and
restarts every heavy lane the PR selected. First decide whose failure it
is: if the latest push run on `main` fails the same way, it is a main
failure (`oteryn-platform-dev` `references/ci-merge.md`, *Reading a red check*).

## 1. Build a tight loop that goes red

Find one command, run it, and show its output, that:

- goes **red** on this exact symptom (the same test name, panic or
  assertion as the CI log), not on a nearby failure;
- is **tight**: seconds, not the workspace. From `portal-rs/`:
  `cargo test --locked -p oteryn-<crate> --test <target> <filter>`, or
  `--lib <filter>` for a unit test;
- runs unattended.

Ways to get one, in order: the failing test itself, narrowed; a new test at
the seam that reaches the bug; a replay of the captured input (a request
body, a fixture, a row) through the handler; `git bisect run` with the loop
when it broke between two known commits.

Read the whole error first: the leaf job log (search it for `error[`,
`panicked at`, `FAILED`), the full panic and the assertion's values.

## 2. Flaky: raise the reproduction rate

- **Timing:** run the loop 50-100 times
  (`for i in $(seq 100); do cargo test ... -q || break; done`), add load
  (`--test-threads` up, parallel runs), and look for a fixed sleep, a missing
  readiness wait or a deadline that is too short.
- **State:** a PostgreSQL test that shares a schema, rows or a global with
  another. Run it alone, then with its neighbours (`--test-threads=1` vs
  default); the one that turns it red is the polluter. Each PostgreSQL test
  needs its own schema.
- **Environment:** `cargo test` runs tests of one binary as threads in one
  process, so process-global state (env vars, statics) leaks between them.

Push the failure rate up until most runs fail; only then form hypotheses.

## 3. Hypotheses, then one change at a time

Write 3-5 ranked hypotheses, each with a prediction: "if X is the cause,
then changing Y makes the loop green". Test the top one by changing one
variable. Instrument only to split hypotheses, and tag every debug line with
one prefix (`[DBG-<task>]`) so one grep removes them all.

## 4. Fix and keep the proof

- Fix the cause, not the symptom: a longer timeout, a retry or a sleep goes
  in only when the loop showed it is the cause.
- The loop becomes the regression test: red before the fix, green after.
  Remove the tagged debug lines.
- If `main` has the same failure, one main fix, landed first.
- Three failed fixes in a row mean the hypothesis or the design is wrong:
  stop, write down what the attempts ruled out, and re-plan.

## 5. Keep the lesson

Note the failure and its root cause in the report and in the task
checkpoint's `first_failure` / `rejected_hypotheses`; the retro
(`oteryn-platform-waves`, *Close the wave*) decides whether it becomes a
check or a skill line.

## 6. Red main: targeted fix

A red push run on `main` blocks every merge, so the loop must run in
minutes, not CI cycles.

1. **Bisect the CI history before reading code.** For the failing job, find
   the last green and first red commit on `main`
   (`scripts/main-health.sh <workflow> 30` in `oteryn-platform-dev`). The
   Merge Queue runs only `CI`, so a `Portal RS CI` failure on `main` is
   usually two PRs that were green alone (`ci-merge.md`, *Blind spot*): look
   at the merges between the two runs first.
2. **Read the last meaningful log line before the symptom**, not only the
   symptom.
3. **Reproduce locally**, red before and green after, on the merged tree.
4. **Scope the fix by the evidence**: the culprit SHA, the failing line and
   the local command go into the fix PR.
5. **Revert is the fallback**: offer the owner a revert of the culprit when
   the failure cannot be reproduced within about an hour.

## Known signatures in this repository

| symptom | usual cause |
|---|---|
| PostgreSQL test green locally and in CI, broken in reality | the test printed `SKIPPED`: `PORTAL_TEST_DATABASE_URL` unset; CI has no database |
| `duplicate migration versions in portal-rs/migrations` | two PRs chose one timestamp; the later one renames its own file |
| `migrate` fails on a real database, tests green | SQL that only runs when migrations are applied; apply them to a scratch PostgreSQL 18.6 |
| contract test byte mismatch | key order, number or whitespace drift in the canonical encoder, or a fixture edited by hand |
| `--locked` fails: lock file needs to be updated | `Cargo.lock` not regenerated after a merge or a new dependency (`ci-merge.md`, *Merge conflicts*) |
| clippy red in CI only | local run without `--all-targets` or on one crate only |
| every PR red at `runtime-tests` / *Audit Composer dependencies* | a new advisory in the legacy Laravel lock; one lock bump PR on `main` |
| `platform-gate` red | open the leaf job; the gate only aggregates |
| linker `Bus error` or `No space left on device` | disk full: free the target dir and rebuild; not a code failure |

## Sources

Adapted, not copied verbatim: the Oteryn-Game `oteryn-debugging` skill;
mattpocock/skills `diagnosing-bugs` (MIT); obra/superpowers
`systematic-debugging` (MIT); addyosmani/agent-skills
`debugging-and-error-recovery` (MIT).
