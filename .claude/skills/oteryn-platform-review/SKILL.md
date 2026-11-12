---
name: oteryn-platform-review
description: Reviewing an Oteryn-Platform diff before it lands (a returned subagent or Codex result, your own branch before ready, several PRs landing together) and answering review findings from Codex, Claude review or a person. Load before marking a change ready, when asked for a review, and when review comments arrive.
---

# Reviewing changes in Oteryn-Platform

A review reads code, not reports. The worker's summary, the PR body and a
green check are claims; the diff and the merged tree are the evidence.

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

- `P0` (security, data or money loss, broken `main`) and `P1` (wrong
  behaviour a real caller reaches) block landing and name the concrete
  path: the input or state, the caller that reaches it, the wrong result.
  Without a reachable caller it is at most `P2`.
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

- **Batch review.** One review is one round of parallel lens runs over the
  same head. Each run is told to report every finding, with no cap on lines
  or count, and every other instance of the same class in the diff.
- **Read `references/batch-review.md` first.** It covers the lenses, chunking
  of big diffs, the prompt, the merge, verify and sweep steps, the single
  batch fix and the stop rule. Read it before you start a review or delegate
  one.
- **Limits.** At most 2 review rounds per PR. The review never edits code.
- **The output.** It is findings to verify with step 3, not a verdict. A
  failed or empty run means "not reviewed".
- **Acceptance** belongs to the owner and the reviewer, never to the author.

## 6. Receiving findings

- Verify each finding against the code: trace a real caller or input to
  the failure.
- Real and in proportion: fix it in the next push and reply in one line
  with the fix. Not real: reply with the evidence (the path traced, the
  test that covers it). A P1 is never left silent.
- No performative agreement; actions and evidence only.
- Collect all fixes of one review round into one push.

## Sources

Adapted, not copied verbatim: the Oteryn-Game `oteryn-review` skill;
getsentry/skills `find-bugs` (Apache-2.0); obra/superpowers
`receiving-code-review` (MIT); root `AGENTS.md`, *Security and data
invariants*; `portal-rs/README.md`.
