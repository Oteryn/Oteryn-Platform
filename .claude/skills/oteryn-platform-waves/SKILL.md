---
name: oteryn-platform-waves
description: Planning and running several portal-rs plan tasks as a wave in Oteryn-Platform - choosing tasks from the plan's dependency table, checking that parallel units share no files, migration versions or database, pre-assigning migration timestamps, writing delegation packets for Codex or subagents, landing the PRs through the Merge Queue, and closing a wave with a retro. Load when planning more than one change, delegating, or landing several PRs.
---

# Planning and landing waves

A wave is a few plan tasks built in parallel and landed one after another.
Parallel work is fast only while the units truly share nothing; the
Merge Queue does not run `Portal RS CI`, so two units that collide are
found only on `main` (`oteryn-platform-dev` `references/ci-merge.md`,
*Blind spot*).

## 1. Choose and slice

- Take the work from plan §10 (`docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`):
  only tasks whose *Zależy od* (depends on) tasks are merged, and whose
  Game-side prerequisites in plan §6 are in place or not needed.
- Skip a task whose contract is `PROPOSED`/`CANDIDATE` where the plan says
  not to implement from it yet; it waits, it is not guessed.
- One task = one branch = one PR. A task too large for one PR splits into
  rows the plan names (`T08`, `T08b`, ...); propose the split, do not invent
  task ids.
- Each unit is a thin path through every layer it touches (migration,
  repository, handler, template, test), not one layer done completely.

## 2. Check that the wave can run in parallel

For each unit list the files it will touch (the plan row's paths), then
look past the list:

- **hot files** no two units edit beyond one added line: the router in
  `crates/web` (`lib.rs`, `surface.rs`), workspace `portal-rs/Cargo.toml`,
  `templates/base.html` and shared components, `static/` tokens;
- **hidden shared surfaces:** `Cargo.lock` (two units adding crates),
  migration versions, shared crates (`core`, `db`, `web`, `internal-api`),
  one fixture directory, and one runtime resource (each lane gets its own
  PostgreSQL database or schema);
- **contracts:** two units that change one type, trait or wire message
  contend even in different files.

Units that contend move to the next wave; doubt about one unit never holds
back the rest.

## 3. Pre-assign migration versions centrally

Give each unit that needs a migration its exact file name
(`<UTC timestamp>_<task>_<what>.sql`), with timestamps checked against
`portal-rs/migrations/` on `main` and in open PRs
(`gh pr list --state open --json files` or the PR file lists). No worker
picks its own timestamp.

## 4. Write the packets and dispatch

One self-contained packet per unit: `references/delegation.md` has the
format and the rules for taking results back. Dispatch only as many units
as you can review and land the same day.

## 5. Land the wave

1. Review each returned unit (`oteryn-platform-review`) and push it to its
   own branch; open the PR from the template.
2. Before queueing the next unit, merge it locally onto the current
   `origin/main` and run the portal-rs checks on that tree when it shares any
   hot file, `Cargo.lock` or the migrations directory with a unit already
   landed.
3. Queue one PR at a time; after each merge, watch the `Portal RS CI` push
   run on `main`, then check the remaining PRs for conflicts
   (`git merge-tree --write-tree origin/main <head>`).
4. A unit that goes red after `main` moved: merge `origin/main`, fix the
   unit, not `main`.

## 6. Close the wave

- Each PR's post-merge closeout: archive the task record, release
  ownership, update the governing Issue.
- Retro, once per wave, at most three lessons. Candidates are what cost a
  re-push, a re-run, a review finding or a fix in another PR; keep one only
  if someone reading just the final code would repeat it, and cite the PR or
  run. Each kept lesson ends in exactly one of:
  - a proposed automated check, when a script or test can detect it (the
    migration-version guard of #1721 is one);
  - one line in the skill that owns that kind of work;
  - dropped, with the reason.
  Skills change only through a reviewed PR; an agent proposes the line, it
  never edits a skill on its own initiative mid-task.
- Prune at the same time: delete any skill line that changed nothing an
  agent did since the last pruning or contradicts a newer rule.

## Sources

Adapted, not copied verbatim: the Oteryn-Game `oteryn-waves` skill;
EveryInc compound-engineering `ce-work` and `ce-compound` (MIT);
mattpocock/skills `to-tickets`, `writing-for-agents` and `retro` (MIT);
plan §10 and §10.1.
