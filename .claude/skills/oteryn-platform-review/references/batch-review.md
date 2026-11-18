# Batch review: catch most defects in one round

Read this before you start an independent review (SKILL.md §5) or hand one
to a subagent or Codex. It explains how one review round finds most of the
defects in a diff at once, instead of 10 findings, then 5, then 3 over
several rounds.

Rounds shrink for three reasons:
- One reviewer with one prompt stops after its first strong findings.
- A line or count cap cuts off the rest.
- Each fix adds new code that the next round then reviews.

The rules below remove those causes. They do not change when a review is
required or who starts it (SKILL.md §5). Three limits hold:
- At most 2 review rounds per PR.
- The review never edits code.
- The batch fix runs at reasoning effort `medium`.

Adapted from the Oteryn-Game `oteryn-review` skill
(`references/batch-review.md`, Game `main` 203f9222, Oteryn/Oteryn-Game#3130;
the gate, verdicts, fix packet and redesign rule at Game commit
`f64e785ee1e3c81361bbe4b958e5b2ee8ba13c2a`).

Before a round starts, the head passes the gate of SKILL.md §0; a head that
fails it is not reviewed. The lens list of a PR is frozen when its first
round starts; the second round runs only the lenses that had confirmed
findings.

## 1. One round = several lens runs over the same head

A round is one pass over one exact head SHA. Inside that round, run
independent read-only runs, one lens each. Each run starts with a clean
context and does not see the other runs' output.

| Lens | Looks for in portal-rs |
|---|---|
| `auth` | identity, sessions, MFA, recovery, `native_security_generation` / `game_auth_generation` fencing, admin RBAC permission and audit row in the same transaction, CSRF on browser mutations, refusal when state is ambiguous, secrets in logs or `Debug` |
| `money` | wallet, store, bazaar, premium/loyalty, payments: one transaction, rows locked in a stable order, idempotency key and replay, overflow and `i64::MAX`, a refusal that rolls back, checks done by the server and not the client |
| `game-boundary` | the `/internal/v1` and `/internal/v2` wire on the mTLS listener: byte-exact against `docs/contracts/fixtures/`, status bodies and codes, SAN identity and the shared overlap check, default-off flags and `OTERYN_ENV` gates, idempotent operation ids, outbox and reconciliation, timeout treated as unknown, the contract status allowing the code |
| `migrations` | new file with a unique version, additive, no edit to a merged migration, triggers and lock order (row, then counter), indexes for new foreign keys and paged reads, sqlx queries parameterized, unbounded reads, `_ =>` mapping an outcome to success |
| `tests` | tests that pass without testing (a PostgreSQL test that prints `SKIPPED`, no assertion on the outcome, a sleep instead of a wait), a missing refusal or bad-input case, a missing red case for a fix, an edited fixture or assertion |

Choose lenses by diff size and by what the diff touches:

- **Up to about 400 changed lines:** 2 lenses, the two that fit the diff best.
- **Larger diffs, or any diff that touches identity, money, the Game wire or
  a migration:** every lens that applies, `tests` always included.
- **Diffs above about 1500 changed lines:** also split into chunks of about
  400–500 lines by file group (one crate or module per chunk). Each chunk
  gets the lenses that fit it. Every chunk still gets the whole diff as
  read-only context, so cross-file defects are not lost.
- **Holistic runs (owner 2026-11-18):** every PR that lands several changes
  together, and every diff split into chunks, also gets two runs over the
  whole diff as one unit in the same round, reporting findings anywhere in
  it:
  - `whole-integration`: how the parts interact on the integrated head:
    shared state, duplicate or conflicting registrations, ordering of
    handlers and migrations, a migration against another part's code,
    contracts and fixtures against the code, generated files against their
    sources.
  - `whole-flows`: each user or money flow the diff changes, end to end
    (request, handler, service, PostgreSQL and back, plus the Game boundary
    where it is touched), with the abuse cases: two requests at once, a
    retry or replayed request, a crash during a write, values at their
    limits.
  Chunk runs alone are not a covered review of such a PR. The delta review
  reruns `whole-integration` over the fix commits together with the code
  they touch.

Where the runs come from:

- Codex: `codex exec` in a read-only sandbox, or a Codex Cloud packet.
- A read-only subagent (`oteryn-platform-ref-reader` or `Explore`).
- On the PC, the shared review script with `--lens <name>` where it serves
  this repository.

Run the lenses in parallel, within the lane limits in the operator manual.

## 2. The prompt: all findings, no cap

Every lens prompt contains:

- The diff against the exact base (`git diff <base>...<head>`), the lens,
  and SKILL.md §3 as the checklist.
- "Report EVERY real defect you find in your lens, P0 to P3. There is no
  limit on lines or on the number of findings. Do not stop after the first
  strong ones."
- "For each finding, also list every other place in the diff with the same
  defect class (the pattern sweep)."
- Format: `P0|P1|P2|P3 path:line - problem - concrete failure path`.
- "Mark uncertain findings with `(uncertain)` instead of dropping them."
- "End with `VERDICT: CLEAN` or `VERDICT: N findings`."
- "Do not edit code."
- When the PR fixes a CI failure: the failing log lines (the test name, the
  panic or assertion, and the last 30 lines of the job). A reviewer that sees
  the failure finds its cause in the diff much faster.

## 3. Merge, verify, sweep

1. Merge the lens outputs. Remove duplicates by `path:line` and defect class.
2. Give every finding a verdict (SKILL.md §6). Verify each P0/P1 against the code (SKILL.md §6): trace a real caller or
   input to the failure.
   - For a finding that is hard to call, run a second check with the
     opposite brief: a fresh run told "try to refute this finding; default
     to refuted when unsure".
   - Keep the finding when the refute attempt fails.
3. Pattern sweep. For every confirmed finding, grep the diff, the files it
   touches and the rest of `portal-rs/` for the same class: the same call,
   the same lock, the same `_ =>`, the same unchecked add, the same missing
   permission check, and every test that asserts the changed value. Verify
   each hit as in step 2 and list it as `file:line`. Hits inside the
   packet's owned paths join the confirmed list; the others are
   `OUT OF SCOPE` with their severity and go to the lead. A class fixed at
   one site and left at another is the main cause of the next round.
4. P2/P3 findings do not start a fix now. They are fixed in the same push
   when cheap, otherwise noted for the module's next task (SKILL.md §4).

## 4. One batch fix, one delta review, one CI cycle

1. Write all confirmed P0/P1 findings, sweep instances included, to one file,
   numbered, one per line as `N. P0|P1 path:line - problem - failure path`.
   Hand that file to one code-writing run at reasoning effort `medium`. Never
   use one fix run per finding. The packet (SKILL.md §7) states:
   - **Statuses.** The report gives every number one status: `FIXED <test>`,
     `NOT REPRODUCED` or `BLOCKED <why>`. Runs handed a list tend to stop
     after the first item; a missing number is visible.
   - **Sweep list.** Every verified sibling site as `file:line`, including
     tests in other crates that assert the same value. Only sites inside
     the packet's owned paths (`crates/<module>`, its migration,
     `templates/<module>/`, its router line in `crates/web`, its
     `docs/agents/tasks/` packet) are in scope; the rest are `OUT OF SCOPE`.
   - **Red test through the real caller.** Per finding, a test that fails
     first: the handler through the `crates/web` router, the repository or
     transaction on real PostgreSQL (no `SKIPPED` line), the worker job, or
     the `crates/internal-api` contract test on `docs/contracts/fixtures/`;
     not only a unit test of the patched helper. A test that cannot fail is
     reported `NOT REPRODUCED` with no code change.
   - **Smallest fix.** The test reruns green; no existing assertion,
     timeout, budget or fixture changes, and nothing outside the listed
     findings.
   - **Class guard.** For a P0/P1 class that recurred across PRs, when it is
     smaller than the next round: one choke-point function and a
     source-scan or invariant test.
   - **Forbidden paths.** A fix diff outside the owned paths is rejected
     whole; `.github/workflows/**`, `scripts/ci/**` and `tests/ci/**` change
     only in a task that owns them.
2. Review the fix diff yourself (SKILL.md §2–§3) and run the local checks
   (`oteryn-platform-dev`, step 5).
3. Push once. One CI cycle covers the whole batch.
4. The second review round covers only the fix diff (the delta). It uses the
   lenses of the confirmed findings, under the same rules.

Code written by Codex or a subagent is reviewed by a new run with a clean
context, never as a continuation of the run that wrote it.

## 5. When to stop

The diff is done when two independent lens runs over the current head find
no P0/P1. Two lenses in the same round count.

If the second round still finds a P0/P1, the design of the change is wrong:
stop and report to the owner. Never start a third round.

### Redesign instead of another patch

Patching stops, before the round limit, when any of these holds (P2/P3 never
trigger it):

- a round finds more P0/P1 than the round before;
- the same P0/P1 class in the same module or flow comes back in a second PR
  or round;
- a P0/P1 is a regression or a second site of an earlier P0/P1 fix.

Then report to the owner with a proposed redesign: a rule the code can
check, such as a transition table with one test per state and event, one
transaction for a durable value, or one shared source for a policy plus a
parity test. It is implemented after the owner agrees, as its own PR with
its own review, never as a further push to a PR whose rounds are used up.

Example of a proposal (a shape, not a claim about today's code): when bid,
settle and cancel defects keep returning in the Bazaar lifecycle, propose a
state machine for `crates/bazaar`:

1. A transition table, one row per (state, event): the next state or the
   refusal. Each row gets one test on real PostgreSQL that drives the event
   through the real caller and asserts the stored state and the refusal.
2. Then one transition function that every writer calls, inside the
   transaction that locks the row, plus a source-scan test that fails when
   any other code in the crate writes the state column.

## 6. For subagents and child sessions

A packet that delegates a review or a fix points to this file and states:

- the base and head SHA, the lenses to run and, for big diffs, the chunks;
- "report every finding, no cap; sweep each class across the diff";
- the failing CI log lines, when there are any;
- "do not edit code in the review run; fixes go through one batch fix run
  after verification".

The report back is the merged, verified list with the sweep instances, not
the raw lens output. A packet's report line limit does not apply to it.
