---
name: oteryn-platform-review
description: Reviewing an Oteryn-Platform diff before it lands (a returned subagent or Codex result, your own branch before ready, several PRs landing together) and answering review findings from Codex, Claude review or a person. Load before marking a change ready, when asked for a review, and when review comments arrive.
---

# Reviewing changes in Oteryn-Platform

A review reads code, not reports. The worker's summary, the PR body and a
green check are claims; the diff and the merged tree are the evidence.

## 0. Gate before any review is requested

Review is for what a command cannot find. Before a review is requested on a
head, that head has:

- the local checks for its changed paths green, with the commands of
  `oteryn-platform-dev` `references/local-checks.md` (which win): for
  `portal-rs/**`, `cargo fmt --all --check`,
  `cargo clippy --locked -q --workspace --all-targets -- -D warnings` and
  `cargo test --locked -p oteryn-<crate>` for each changed crate, with
  `PORTAL_TEST_DATABASE_URL` set to a scratch PostgreSQL 18.6; a
  PostgreSQL test that printed `SKIPPED` was not run;
- the real caller run red before the fix and green after: the handler
  through the `crates/web` router, the repository on real PostgreSQL, the
  worker job, or the `crates/internal-api` contract test against
  `docs/contracts/fixtures/`. Where the session has no PostgreSQL, the
  coordinator runs them on a scratch PostgreSQL 18.6, or, only on a
  ready (non-draft) PR whose head is current with `main` (the PR run tests
  the merge commit), adds the `heavy-tests` label, whose PostgreSQL test
  shards count with the test names taken from the job log; the fast PR lane ("Unit tests without
  PostgreSQL") never counts;
- one frozen head: only the final head SHA is reviewed; a later push makes
  it a new head;
- the touched module's PostgreSQL tests run with their names listed and no
  `SKIPPED`, proving they reached their assertions;
- a sweep (`rg`) of every changed literal, version and count across tests,
  fixtures, contracts and docs, all updated in the same commit;
- after any merge of `main`: a rebuild and the full tests of each changed
  crate on the merged head;
- the author's self-review against §3, at high effort, and for risk
  classes the reviewed design note and consumer map.

Effort: self-review and Codex review run at high; P0/P1 fixes in the risk
classes (`oteryn-platform-coding`, *Before code*) run at high.

A governance-only head (docs, rules, skills, agent definitions, no
`portal-rs/**` change) records the Rust, real-caller and PostgreSQL rows as
`NOT_APPLICABLE: governance-only` (root `AGENTS.md`); the frozen head, the
sweep and the self-review still apply. A workflow change keeps the rows its
own checks need.

A head that fails the gate goes back to its author, not to review.

## 1. Gather the whole input

- The diff against the exact base: `git diff <base>...<head>` (the base SHA
  from the packet or the PR). Page through truncated output until every
  changed line has been seen, and list the files.
- The governing Issue, the plan task row (allowed paths, acceptance), the
  contract the change implements and its fixtures.

## 2. Look beyond the diff summary

- Files outside the task's allowed paths, edits to shared files that are
  not pure additions, and commits nobody asked for.
- Changed limits, timeouts, flags' defaults, rate limits, size bounds and
  lint levels: find who moved each value and why. A flag that now defaults
  on is a finding.
- Edits to test helpers, fixtures or assertions; a test that can print
  `SKIPPED` is not evidence for the path it covers. `#[ignore]`, a removed
  test or a hand-edited fixture blocks landing (step 4).
- Migrations: new file only, unique version, additive, no edit to a merged
  one; indexes for new foreign keys and paged reads.
- `Cargo.toml` / `Cargo.lock`: a new third-party crate is explained in the
  PR (maintenance and security, `CONTRIBUTING.md`).

## 3. Check every item for every file

- **Authorization:** every mutation checks the actor server-side; admin
  routes check the permission (`crates/admin` RBAC) and write audit.
- **Browser mutations:** CSRF and session layer present; cookies keep their
  attributes.
- **Input:** bounds, size limits and overflow on every untrusted value
  (form, JSON, path, header); parameterized SQL only.
- **Money and scarce goods:** one transaction, rows locked in a stable
  order, idempotency key, refusal rolls back.
- **Game boundary:** byte-exact against the fixture, idempotent with an
  operation id, outbox and reconciliation, timeout treated as unknown,
  default-off flag, the contract's status allows implementation.
- **Secrets:** no URL, token or key in logs, errors, `Debug` or templates;
  constant-time comparison for secrets.
- **Unbounded reads:** every list has a limit or page and an index.
- **Async:** no lock guard held across `.await`; no blocking call on the
  runtime.
- **Output:** askama escaping intact; Markdown sanitized.

For each possible finding, confirm it is real: not handled elsewhere in
the diff, no test covers it, the surrounding code agrees.

## 4. Audit, then findings

First note which files you read, each item of step 3 with clean or not, and
what you could not verify and why. Then the findings:

- Severity rubric (owner decision 2026-11-25, shared with Oteryn-Game);
  every reviewer (Codex sol and astra, Claude review, a person) grades by
  the effect on a real caller, not the size of the code:

  | Grade | What it is | Examples |
  | --- | --- | --- |
  | P0 | Security, money or data damage, broken `main` | auth, MFA or permission bypass; wallet, store or payment value lost or duplicated; data loss; a red `main` build or gate |
  | P1 | A real caller reaches a wrong result: a user, staff, the game boundary, or a tool, workflow, gate or agent definition the repository runs | a flow that cannot finish or gets stuck; a wrong `/internal/v1` response; a migration or guard that rejects valid use; a gate that passes a PR it must hold; an agent or workflow file that does not parse |
  | P2 | A contained defect or copy/text; behaviour still works | wrong text or label; stale docs or counts; a test that reads a retired file, while no gate or `main` job turns red |
  | P3 | Style and hygiene | naming, duplicated check, readability |

  P0 and P1 block landing and name the concrete path: the input or state,
  the caller that reaches it (`path:line`), the wrong result. Hard rule: a
  finding without the `path:line` of a real caller (a user, staff path, the
  game boundary, a tool, workflow, gate or agent the repository runs, or CI
  for a broken `main`) is at most `P2`.
- A skipped, ignored, loosened or deleted test, a hand-edited fixture or an
  edited merged migration breaks a repository rule and blocks landing
  whatever its label.
- `P2` and lower never block: fixed in the same push when cheap, otherwise
  noted for the module's next task.
- Each finding: `file:line`, the defect class with every instance of it in
  the diff, the evidence it is real, and a concrete fix. No style points,
  praise or retelling of the diff.
- Nothing significant found: say so in one line.

## 5. Independent review

A second model or person reviews only where the governing task, the bound
META policy or a contract's acceptance rule requires it (several contracts
stay `PROPOSED` until an exact-head independent review). The coordinator
starts it, once per frozen head, after the local checks pass, read-only,
with the diff and step 3 as the checklist.

- **Batch review.** One review is the runs `references/batch-review.md` §1
  chooses for one head: ONE holistic read-only Codex Cloud run over the
  whole diff and the callers outside it, model `gpt-6.2-sol` at effort
  `high` (above about 1500 code lines, sol area runs replace it); a PR in a risk class, one changing docs, rules,
  skills, workflows or agent definitions, or a code diff above about 1500
  lines also gets one `gpt-6.2-astra` run at `high` on the same head, whose P0/P1 count only after a traced REAL
  verdict (run table: `references/batch-review.md` §1). Never at `medium`; no lens runs; runs split by area only above about 1500
  lines. Each P0/P1 cites `path:line` for input, caller and wrong result and
  is self-refuted before output; an unsettled one is `P2 (uncertain)`.
  Launched, polled and read from the session's own chat, with model and
  effort set explicitly; never through the PC.
- **Read `references/batch-review.md` first.** It covers the holistic run, chunking
  of big diffs, the prompt, the merge, verify and sweep steps, the single
  batch fix and the stop rule. Read it before you start a review or delegate
  one.
- **Limits.** At most 2 review rounds per PR. The review never edits code.
- **The output.** It is findings to verify with step 3, not a verdict. A
  failed or empty run means "not reviewed".
- **Acceptance** belongs to the owner and the reviewer, never to the author.

## 6. Receiving findings

- Give every finding one verdict, with `file:line`, before any fix:
  - `REAL-P0`..`REAL-P3`: a real defect; a `REAL-P0/P1` names the reachable
    caller you traced;
  - `BY DESIGN`: the code does what the plan, contract or task says, and
    you cite where;
  - `DUPLICATE`: the same defect as an earlier finding, which you name;
  - `FALSE`: not a defect; name the trace or test that disproved it;
  - `OUT OF SCOPE`: the fix needs a path outside the branch's owned paths;
    it keeps its severity and goes to the owning task or lead (a P0/P1 as
    a blocker), never into this fix run.
- Verify each finding against the code: trace a real caller or input to
  the failure.
- Real and in proportion: fix it in the next push and reply in one line
  with the fix. Not real: reply with the evidence (the path traced, the
  test that covers it). A P1 is never left silent.
- No performative agreement; actions and evidence only.
- Collect all fixes of one review round into one push.

## 7. The fix packet

One packet for all confirmed findings of a round, never one per finding
(`references/batch-review.md` §4):

- the findings numbered, each needing a status in the report:
  `FIXED <test>`, `NOT REPRODUCED` or `BLOCKED <why>`;
- the sweep list: every sibling site of each class as `file:line`; sites
  inside the packet's owned paths are fixed, the others are `OUT OF SCOPE`;
- a red test first, through the real caller (§0), not only a unit test of
  the patched helper.

## 8. When a P0/P1 class recurs

When a round finds more P0/P1 than the one before, the same P0/P1 class
comes back in the same module or flow, or a P0/P1 is a regression or a
second site of an earlier fix: stop patching and propose a redesign to the
owner, a rule the code can check. Every recurring P0/P1 class becomes a CI
guard test (a test that fails on any new instance of the class), landed
with or before the fix. It lands as its own PR with its own
review. Details and the example are in `references/batch-review.md` §5.

## 9. Per-PR review and the owner's review loop

§0-§8 are the per-PR review (at most 2 rounds). An owner-requested review
loop reviews a whole area of merged code on `main` in rounds; it never
counts as, or replaces, the review of an open PR, and each fix PR it makes
gets its own per-PR review. Only the coordinator runs it, for a loop the
owner asked for. Read `references/review-loop.md` only when running one;
it also holds the flag boundary rule for off-flag features.

## Sources

Adapted, not copied verbatim: the Oteryn-Game `oteryn-review` skill (§0
and §6-§9 at Game commit `f64e785ee1e3c81361bbe4b958e5b2ee8ba13c2a`);
getsentry/skills `find-bugs` (Apache-2.0); obra/superpowers
`receiving-code-review` (MIT); root `AGENTS.md`, *Security and data
invariants*; `portal-rs/README.md`.
