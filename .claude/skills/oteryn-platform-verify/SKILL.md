---
name: oteryn-platform-verify
description: Evidence before any claim in Oteryn-Platform - before saying done, fixed, passing, green or mergeable, before a report, a commit message, a PR description, a validation-table row or a task checkpoint, run the command that proves it and read its output. Load whenever you are about to report a result or state of work.
---

# Evidence before claims

A claim without fresh output from the command that proves it is a guess.
Workers' reports, CI from an older head and "should pass" are not evidence.

## The gate

Before stating any result:

1. Name the command that proves it.
2. Run it in full, now, on the exact tree or head the claim is about.
3. Read the whole output: exit code, failure count, the test names, and
   every `SKIPPED` line.
4. State the result with the evidence (command, head SHA, counts); if the
   output disagrees with the claim, report what it shows instead.

| claim | needs | not enough |
|---|---|---|
| tests pass | `cargo test --locked` output with 0 failed, on this tree | an earlier run, a run on another head |
| PostgreSQL path works | the test run with `PORTAL_TEST_DATABASE_URL` set and no `SKIPPED` line for it | a green `Portal RS CI` (it has no database) |
| migrations apply | `oteryn-portal migrate` on a scratch PostgreSQL 18.6 plus `-p oteryn-db --test migrations` | the files compile |
| Game contract kept | the contract test comparing bytes with `docs/contracts/fixtures/` | the struct looks the same |
| the bug is fixed | the loop that was red on the bug, now green | code changed, "looks right" |
| a regression test works | seen red without the fix and green with it | it passes once |
| CI is green | every check the PR started, on the current head SHA | a green run on an older head |
| mergeable | `oteryn-platform-dev` `references/ci-merge.md`, *Merge conditions*, on the current head | the PR looked fine an hour ago |
| a worker finished | the diff in the tree and your own run of its checks | the worker's report |

## Words

- PR validation-table rows and checkpoint `validation` entries use
  `PASS`, `FAIL`, `BLOCKED`, `NOT_RUN` or `NOT_APPLICABLE` with a concrete
  reason (`docs/agents/tasks/TASK_TEMPLATE.md`); a skipped PostgreSQL test
  is `NOT_RUN`, never `PASS`.
- Numbers only when counted, with what was counted.
- A check you could not run is reported as not run, with the reason; it is
  never folded into "passes".

## Sources

Adapted, not copied verbatim: the Oteryn-Game `oteryn-verify` skill;
obra/superpowers `verification-before-completion` (MIT).
