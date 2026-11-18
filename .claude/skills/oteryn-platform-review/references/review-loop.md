# The owner's review loop: finding logic errors across a whole area

Read this only when running a review loop the owner asked for (SKILL.md §9).
Only the coordinator runs a loop, and only for a loop the owner asked for;
this file grants nothing. A per-PR review follows SKILL.md §0-§8 and
`batch-review.md` instead.

Adapted from the Oteryn-Game `oteryn-review` skill
(`references/review-loop.md` at Game commit
`f64e785ee1e3c81361bbe4b958e5b2ee8ba13c2a`).

An area of portal-rs is larger than one review run can read, so a loop does
not try to read "all the code". It reviews the code through cuts that each
follow one rule across every crate that touches it. Logic errors live
between modules; a file-by-file read misses them.

## 1. Charter (written before round 1, approved by the owner)

The charter lives in the loop's packet under `docs/agents/tasks/`
(`docs/agents/tasks/TASK_TEMPLATE.md`) and does not change between rounds,
so rounds are comparable:

- **Scope:** the area (for example the Bazaar), its crates
  (`crates/<module>` and the crates it calls), its tables and migrations
  (`portal-rs/migrations/`), its routes in the `crates/web` router, its
  workers and its `/internal/v1` routes.
- **Stop bar and frozen lens list:** the bar of section 5 and the lenses
  every round runs (`batch-review.md` §1). Neither changes during the loop.
- **Rules (invariants):** the behaviour the code must have, taken from
  written sources only: root `AGENTS.md` (*Security and data invariants*),
  `docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`, `docs/contracts/` and its
  fixtures, and SKILL.md §3. A rule with no written source is a batched
  owner question, never a reviewer's guess.
- **Map:** for each state the area owns (a table, a ledger, a lifecycle)
  every writer, built by commands (`rg` for SQL writes to the table, for
  the router and worker registration, and for the public functions that
  call them), one row per writer with `file:line`. The coordinator
  spot-checks it. It is an input, not evidence: reviewers report where the
  code disagrees with it.
- **Cuts and budget:** the runs of one round (section 2) and the round
  budget: 3 rounds. When the third round has not met the stop bar, the loop
  stops and the coordinator reports the trend to the owner.

## 2. One round: three cuts, chunks as the baseline

All runs of a round review one exact head of `main`, started only when every
fix PR of the previous round has merged. They run in parallel, read-only,
under `batch-review.md` §1-§2 (every finding, no cap, sweep each class,
SKILL.md §3 as the checklist).

| Cut | One run per | The run gets | It must return |
|---|---|---|---|
| State | rule, or lifecycle table | the rule and the writer list from the map | a table: writer, `file:line`, holds or breaks, and the path when it breaks |
| Flow | end-to-end flow (for the Bazaar: list, bid, settle, cancel, recover after a crash) | the flow's entry point (route, worker job or internal route) and the map | the trace across crates, step by step, and each break; the brief includes the hostile cases: two requests at once, a replayed request, a disconnect midway, a restart during a write |
| Boundary | contract (browser and server, Platform and Game) | the Platform code, the contract in `docs/contracts/` and its fixtures; Game code is not read | each field and outcome where the code and the contract disagree |

A writer, step or field missing from a returned table means "not reviewed",
not clean. Chunk runs (`batch-review.md` §1, about 400-500 lines of one
crate) cover only the files no cut reached, and the files a cut flagged.

## 3. Findings ledger and synthesis

- One ledger per loop, in the loop's packet, lists every finding ever
  reported, with its verdict (SKILL.md §6) and `file:line`. Every run gets
  it as "known; do not re-report unless the named code changed". New
  findings are deduplicated against the whole ledger. Only new confirmed
  findings count toward the stop bar and the trend.
- After the merge of a round's results, one synthesis step groups the
  confirmed findings by root cause and lists each state, flow or boundary
  no run covered. A group of P0/P1 with one root cause goes to SKILL.md §8;
  an uncovered item gets a run in the next round.

## 4. Between rounds

- Verdicts per SKILL.md §6, sweep per `batch-review.md` §3.
- A P0 gets its own fix PR at once. Other confirmed P0/P1 go into a few fix
  PRs grouped by area, with disjoint files so they can run in parallel. P2
  go into one grouped P2 PR per round, opened after that round's P0/P1 PRs
  merge. Each fix PR is an ordinary PR with its own per-PR review and a fix
  packet (SKILL.md §7).
- Each confirmed logic error gets a test through the real caller (SKILL.md
  §0) that fails before the fix.
- Out-of-scope findings go to their owning task or lead with their
  severity.

## 5. Stop

- **Done:** two consecutive rounds on the same head of `main`, with the
  frozen lens list, each with zero new confirmed P0, P1 and P2 on
  v1-reachable code, and every run of both finished. The second round uses
  a different cut (by flow instead of by state, or refute and completeness
  briefs).
- **Not converging:** the same P0/P1 class back in the same module or flow,
  or a regression of an earlier fix, sends that area to SKILL.md §8 before
  the next round.
- **Budget:** 3 rounds (section 1). The loop then ends with a report.

## 6. The flag boundary rule

Every game- or money-touching feature sits behind a flag, off by default
(`oteryn-platform-dev`). The loop treats code by what a v1 user can reach:

- **v1-reachable code** is the code reachable with every flag at its
  default. A loop that leaves off-flag code at a lower bar needs a test
  proving that with the flags off there is no route in the `crates/web`
  router or the internal API, no scheduled job, no worker and no write to
  the feature's tables. Without that test the code counts as v1-reachable.
- **Inside an off-flag feature** only P0 and P1 count toward the stop bar;
  P2 and P3 go to the feature's backlog.
- **Before a flag is switched on** (default changed, or enabled in any
  environment), the feature gets one full round, every lens and every cut
  that applies, with zero findings over the v1 stop bar.
