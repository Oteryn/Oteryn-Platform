# Delegating work and taking it back

A delegated agent knows only its packet. Most rework comes from packets
that leave out the base, the migration version or the definition of done,
and from returned diffs pushed without review.

## How much to delegate at once

Send no more tasks than results you can review and land the same day.
Parked work stays on its branch with a line in the governing Issue, not as
an open PR.

## The packet

One self-contained text. Plan §10.1 is the base template; include, in this
order:

1. **Task:** plan id and name, governing Issue, one-sentence goal, plan
   sections to read (by number).
2. **Base:** the exact `main` SHA and the branch name
   (`codex/portal-rs-<ID>` for Codex, a task branch for a subagent). The
   worker's first step is to check that `HEAD` equals that SHA and to stop
   and report on a mismatch.
3. **Migration:** the exact file name assigned centrally, or "none".
4. **Allowed paths:** from the plan row; shared files only by adding a
   line. Everything else is read-only.
5. **Contract:** the contract file and fixture directory, the contract's
   status, and the flag the feature sits behind (default off).
6. **Success check:** the test that is red on the base and must be green,
   named exactly, plus from `portal-rs/`:
   `cargo fmt --all --check`, `cargo clippy --locked --all-targets -- -D warnings`,
   `cargo test --locked`. Tests cover success, refusal and bad input. A
   PostgreSQL check only when the packet gives a database route; otherwise
   the coordinator runs it.
7. **Report:** at most 15 lines: result, head SHA, commands with pass/fail
   counts and any `SKIPPED` tests, open questions with options and a
   recommendation, what is not done.

Keep the packet short and decided: name the files, types, functions,
values from the contract and the test for each step, and leave open only
what any competent engineer would choose the same way. Point at repository
files instead of pasting them. Repository subagents use the definitions in
`.claude/agents/`; their tier follows the route's *Tier* column in
`docs/agents/CONTEXT_ROUTING.md`, and the model for a tier comes only from
`docs/agents/META_AGENT_POLICY_BINDING.json`.

## Limits to paste into every packet

The receiving agent cannot see this skill, so paste these lines verbatim:

- No changes outside the allowed paths; no `.github/workflows`; no new
  third-party crate unless the packet names it.
- No `unwrap`, `expect` or `panic` outside tests; every game- or
  money-touching feature behind a flag, off by default.
- Never weaken, skip or delete a test; never edit a fixture under
  `docs/contracts/fixtures/` to match the code; never edit a merged
  migration.
- Never read credentials, tokens or `.env` files; never print a database URL.
- Do not read or change Oteryn-Game or any other repository.
- Do not commit notes, logs, scratch output or working files.
- Push only the packet's own branch and only when the packet says so; never
  merge, label or enqueue.

## Taking the result back

Before a returned diff goes onto a branch:

1. Check it applies on the packet's base, then against current `main` with
   `git merge-tree --write-tree origin/main HEAD`; merge only on a conflict.
2. Read the full diff. Reject or trim: files outside the allowed paths,
   stray notes, weakened assertions, `#[ignore]`, edited fixtures, renamed
   migrations, unrelated refactors, a flag defaulting on.
3. Check the migration version is still free on `main` and in open PRs.
4. Run the success check yourself, including the PostgreSQL tests with a
   database; the agent's report is a claim (`oteryn-platform-verify`).
5. Review it (`oteryn-platform-review`), push one commit, then open or
   update the PR from the template.

A task that died without a result is restarted from the same packet, not
silently dropped. Track every delegated task until its result is merged,
rejected or explicitly parked.
