---
task_id: OTERYN-20261013-optional-ui-skills
governing_issue: 1602
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261013-optional-ui-skills

## Goal

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1602

Install project-local design, UI-review and browser-testing skills behind the task-scoped user grant in root AGENTS.md. Select the smallest relevant set after permission; keep required checks unaffected.

## Acceptance criteria

- [x] Explicit grant, continuation, revocation, named-skill scope and worker handoff rules.
- [x] Pinned sources, SHA-256 provenance and retained Apache-2.0/MIT licenses.
- [x] Laravel/Blade-specific wrappers and browser helper limits.
- [x] Focused validation, exact-head CI and governed integration of primary PR #1603 completed; this archive is the terminal closeout.

## Ownership

```yaml
owned_paths:
  - AGENTS.md
  - .agents/skills/**
  - docs/agents/tasks/active/OTERYN-20261013-optional-ui-skills.md
  - docs/agents/tasks/active/OTERYN-20261013-optional-ui-skills.md
modules:
  - agent-governance
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game issue 2080; separately authorized in the same owner chat
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-13T18:51:11.363279Z
head: bbc43af6052465ea49a138fb95765576d470d83e
branch: codex/optional-ui-skills-20261013
pr: 1603
status: completed
context_routes:
  - agent-governance
owned_paths:
  - AGENTS.md
  - .agents/skills/**
  - docs/agents/tasks/active/OTERYN-20261013-optional-ui-skills.md
  - docs/agents/tasks/active/OTERYN-20261013-optional-ui-skills.md
proven:
  - Sources are pinned and licenses retained.
  - PR 1603 is merged with successful real merge-group platform-gate.
  - Native registry discovery and explicit bootstrap/catalog decision trials passed.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - AGENTS.md
  - .agents/skills/**
  - docs/agents/tasks/active/OTERYN-20261013-optional-ui-skills.md
validation:
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: No runtime UI, dependency, security or application code changes.
blockers:
  - none
next_action: Deliver this terminal archive through the bounded closeout PR; no remaining skill implementation.
```

## Review and evidence

Independent AI review not selected: optional-aid instructions narrow selection without changing autonomous write/integration authority, runtime authorization or workflow gates. Self-review and deterministic checks required. The catalog and evaluation.json distinguish deterministic checks, two actual model trials in this repository, native registry discovery and the blocked full CLI model startup. The paired candidate covered 12 permission-selection cases; no universal enforcement or visual-quality claim is made.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Ordinary same-repository delivery; verify deletion after merge.
source_branch_evidence: PR 1603 merged; original source ref readback returned 404 after protected-main verification.
```

## Validation and terminal disposition

- Checkpoint and Documentation/Agent IA validation: PASS.
- `OTERYN_META_POLICY_ROOT=/workspace/oteryn-meta-policy python -m unittest discover -s tools/agents -p 'test_*.py' -q`: PASS, 158 tests.
- `python tools/validation/prompt_eval.py`: PASS, deterministic contract only; model trials 0.
- `python tools/agents/policy_consistency.py --meta-root /workspace/oteryn-meta-policy`: PASS against authenticated bound META revision.
- Source SHA-256/license checks and `git diff --check`: PASS.
- Changed-file self-review: PASS.

PR: https://github.com/Oteryn/Oteryn-Platform/pull/1603 . PR #1603 is verified terminal. This bounded closeout archives its packet after merge, preserving the actual final evidence and releasing implementation ownership. Exact final head bbc43af6052465ea49a138fb95765576d470d83e is delivered by merge commit 87f3709af17cc4bee3b3597d06dac9a9581cbc81. Source deletion and protected-main readback were verified. Runtime E2E is NOT_APPLICABLE because only optional agent aids are added. Explicit bootstrap/catalog model qualification passed: paired baseline/candidate trials, 12 decision cases each, candidate 12/12 intended selections. Native registry discovery passed; full CLI model startup remains authentication-blocked, with no universal enforcement claim.

## Bounded CI repair

The initial exact-head Agent Governance run 38442836688 found stale active ownership for already-merged PR #1591. This task also moves only `OTERYN-20261013-platform-pg-p6a-impl.md` from active to archive with verified merge, gate and branch-deletion evidence. No open PR #1598/#1599/#1601 owned that record at the allocation check; no P6 runtime or deploy implementation is modified. Live task liveness is rerun before the repaired head is frozen.

Live task liveness after reconciliation: PASS, 12 active tasks and 0 advisory findings. Both archived checkpoints and the same 158 agent-tool tests, documentation IA and deterministic prompt contracts passed after repair.

## Automatic review repairs

- Kept this packet active until verified terminal PR readback; recorded the explicit archive-pending transition.
- Removed the unmodified upstream server helper because port-only readiness cannot establish process ownership. No fork or new launcher is introduced.
- Normalized trailing whitespace in the upstream browser-testing reference and recorded separate source/local SHA-256 hashes; exact candidate whitespace validation is required, not an unstaged diff on a clean checkout.

Model qualification is recorded in .agents/skills/evaluation.json with the exact root/catalog/entry-point hashes consumed by the trial. Actual model IDs, token/time telemetry and native auto-discovery are not inferred.

## Terminal primary delivery

- PR #1603: merged on 2026-10-13; exact source head `bbc43af6052465ea49a138fb95765576d470d83e`.
- Protected main readback: `87f3709af17cc4bee3b3597d06dac9a9581cbc81`, the verified primary merge commit.
- Governed executor: run `38448802038`, control comment `6152617325`, accepted UUID `1ee97ce5-bf57-4169-9f57-5ba401a532db`; receipt sequence 1 and same-UUID/target readback sequence 2.
- Real `merge_group` run `38448875103`: success, including aggregate `platform-gate`, PostgreSQL 16/17 and runtime tests.
- Original source branch: deleted automatically, ref readback 404.
- Deferred P2 `r4262575201`: fixed here by replacing stale model-trial prose with the actual qualified/blocked boundaries.
- Governing Issue #1602 remains open until this terminal archive is merged; primary PR used a reference rather than prematurely closing it.
- No optional-skill content, runtime behavior, dependencies or workflows are changed by this closeout.
