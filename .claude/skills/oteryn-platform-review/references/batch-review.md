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
- The batch fix runs at reasoning effort `high` for any fix in a risk class
  (`oteryn-platform-coding`, *Before code*) and `medium` otherwise.

Aligned with the Oteryn-Game `oteryn-review` skill
(`references/batch-review.md` §1-§2 at Game commit
`9db4ccb5c42f6d2f0701a6c485c6b2291d712a8b`, owner decisions 2026-11-25).

Before a round starts, the head passes the gate of SKILL.md §0; a head that
fails it is not reviewed.

## 1. One round = one holistic run over the same head

A round is one pass over one exact head SHA. By default the round is ONE
read-only Codex Cloud run over the whole diff (owner decision 2026-11-25).
The run reads the whole diff as one unit, opens the callers and consumers of
every changed symbol outside the diff, and checks SKILL.md §3, including:

- how the parts interact: shared state, duplicate or conflicting
  registrations, ordering of handlers and migrations, a migration against
  another part's code, contracts and fixtures against the code, generated
  files against their sources;
- each changed user, staff or money flow end to end (request, handler,
  service, PostgreSQL and back, plus the `/internal/v1` boundary where it is
  touched), with two requests at once, a retry or replayed request, a crash
  during a write and values at their limits;
- the risk areas: identity, sessions and MFA fencing; admin RBAC with the
  audit row in the same transaction; wallet, store, bazaar and payments
  (one transaction, stable lock order, idempotency, overflow); the Game wire
  byte-exact against `docs/contracts/fixtures/`; migrations (unique version,
  additive, no edit to a merged one); tests that pass without testing.

There are no per-lens runs. Split only when the diff is above about 1500
changed lines: chunks of about 400-500 lines by file group (one crate or
module per chunk), each chunk run with the whole diff as read-only context,
plus the one holistic whole-diff run in the same round. Chunk runs alone are
not a covered review. The second review (delta) is one holistic run over the
fix commits and the code they touch.

**Model and effort.** Every review run is a Codex Cloud task, read-only, at
reasoning effort `high`, with the runs chosen from the table below; the runs
of one round on one head are the one covered review of that head.

### Choosing the runs (owner decision 2026-11-25, aligned with Oteryn-Game)

"Lines" are hand-written changed lines (generated data, pins and lockfiles
excluded). Risk classes: identity, sessions or MFA; game tickets or
admission grants; the `/internal/v1` wire with Oteryn-Game; wallet, store,
bazaar or payments; migrations; authorization (the same list as `oteryn-platform-coding`,
*Before code*).

| Situation | Runs of the round |
| --- | --- |
| Code PR up to about 1500 lines, no risk class | `gpt-6.2-sol` high, one holistic run |
| Code, contract or fixture change in a risk class | sol high + `gpt-6.2-astra` high on the same head, same prompt |
| Code diff above about 1500 lines (code lines only; a docs-only diff of any size uses the docs row) | sol high per chunk plus the holistic sol run + one astra high whole-diff integration run |
| Docs, rules, skills, workflows or agent definitions | sol high + astra high (calibration: astra found real P1s on a rules PR that sol missed) |
| Delta review (second round) | sol high over the fix commits and the code they touch; + astra high on the new head whenever the PR is in a risk class (§4) |
| CI-failure fix | sol high with the failing log lines in the prompt |

When rows overlap, run the union of their runs. Never astra alone and never
effort `medium`. An astra P0/P1 counts only after the review
owner traced it (input, caller, wrong result) to a REAL verdict (§3); one
disproved is recorded FALSE and closed, one that cannot be traced either way
goes to the P2 backlog. Never review at effort `medium`. Name the task
`review-<tag>` (`review-<tag>-astra` for the second run).

**Launch route (owner rule 2026-11-25).** Every session launches, polls and
reads its Codex Cloud tasks from its own chat, with its own `codex login`
and the Platform Codex Cloud launcher (`submit`, `status`, `poll`, `send`,
`list`). The owner's PC is never a relay: no PC wrapper, `codex exec` on the
PC or review script on the PC starts a task for a cloud session. Every
`submit` and `send` sets the model and effort explicitly (the launcher reads
`OTERYN_CODEX_MODEL` and `OTERYN_CODEX_EFFORT`); never rely on a default. A
session whose login fails (HTTP 401) reports it to the coordinator and does
not reroute.

## 2. The prompt: all findings, no cap

Every review prompt contains:

- The diff against the exact base (`git diff <base>...<head>`), SKILL.md
  §3 as the checklist and the SKILL.md §4 severity rubric for the grades;
  the instruction to open callers outside the diff.
- "Report EVERY real defect you find in the diff, P0 to P3. There is no
  limit on lines or on the number of findings. Do not stop after the first
  strong ones."
- "For each finding, also list every other place in the diff with the same
  defect class (the pattern sweep)."
- Format: `P0|P1|P2|P3 path:line - problem - concrete failure path`.
- "A P0 or P1 states its evidence: `path:line` of the input or state, of the
  caller that reaches the changed code, and of the line that gives the wrong
  result. Without a caller you opened and quoted, it is at most P2."
- "Before the output, try to refute each of your P0/P1: look for the guard,
  check, lock or caller that prevents the failure. Drop one you refuted;
  report one you cannot settle as `P2 (uncertain)`."
- "Mark other uncertain findings with `(uncertain)` instead of dropping
  them."
- "End with `VERDICT: CLEAN` or `VERDICT: N findings`."
- "Do not edit code."
- When the PR fixes a CI failure: the failing log lines (the test name, the
  panic or assertion, and the last 30 lines of the job). A reviewer that sees
  the failure finds its cause in the diff much faster.

## 3. Merge, verify, sweep

1. Merge the run outputs (the sol run, the chunk runs for a diff above about
   1500 lines, and the astra runs the §1 table needs). Remove duplicates by `path:line` and defect class.
2. Give every finding a verdict (SKILL.md §6). Verify each P0/P1 against the code (SKILL.md §6): trace a real caller or
   input to the failure.
   - For a finding that is hard to call, the coordinator traces it to the
     owning code; still unsettled, it becomes P2 (uncertain). No extra
     refute run.
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
   Hand that file to one code-writing run at reasoning effort `high` for a
   fix in a risk class, `medium` otherwise. Never
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
     not only a unit test of the patched helper. For a tool, workflow, gate
     or agent-definition defect the reproducer is the parser, script or job
     that loads it, on the packet's owned paths. A test that cannot fail is
     reported `NOT REPRODUCED` with no code change.
   - **Smallest fix.** The test reruns green; no existing assertion,
     timeout, budget or fixture changes, and nothing outside the listed
     findings.
   - **Class guard.** Mandatory for a P0/P1 class that recurred across PRs:
     one choke-point function and a source-scan or invariant test.
   - **Forbidden paths.** A fix diff outside the owned paths is rejected
     whole; `.github/workflows/**`, `scripts/ci/**` and `tests/ci/**` change
     only in a task that owns them.
2. Review the fix diff yourself (SKILL.md §2–§3) and run the local checks
   (`oteryn-platform-dev`, step 5).
3. Push once. One CI cycle covers the whole batch.
4. The second review round covers only the fix diff (the delta): the §1
   table applied to the new head, i.e. one holistic sol run at high over the
   fix commits and the code they touch, plus the astra run on the new head
   whenever the PR's rows need astra (a risk class, or docs, rules, skills,
   workflows or agent definitions); §1 needs both models on the same head.

Code written by Codex or a subagent is reviewed by a new run with a clean
context, never as a continuation of the run that wrote it.

## 5. When to stop

The diff is done when the covered review of the current head (the sol run,
its chunk runs when the diff needed them, plus the astra runs the §1 table needs)
finds no P0/P1 that verified REAL, and no finding that breaks a repository
rule (SKILL.md §4: weakened test, hand-edited fixture, edited merged
migration) or a P0/P1 marked `OUT OF SCOPE` is still open; those block
landing at any grade and go into the fix list.

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
parity test, always with a guard test for the class. It is implemented after the owner agrees, as its own PR with
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

- the base and head SHA, the model and effort of each run (sol high; astra
  high where the §1 run table needs it) and, for diffs above about 1500 lines, the chunks;
- "report every finding, no cap; sweep each class across the diff";
- the failing CI log lines, when there are any;
- "do not edit code in the review run; fixes go through one batch fix run
  after verification".

The report back is the merged, verified list with the sweep instances, not
the raw run output. A packet's report line limit does not apply to it.
