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
- [x] Focused validation completed; exact-head CI and governed integration remain live PR prerequisites.

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
updated_at: 2026-10-13T17:53:59.601261Z
head: UNKNOWN
branch: codex/optional-ui-skills-20261013
pr: 1603
terminal_pr_policy: archive_pending
status: ready
context_routes:
  - agent-governance
owned_paths:
  - AGENTS.md
  - .agents/skills/**
  - docs/agents/tasks/active/OTERYN-20261013-optional-ui-skills.md
  - docs/agents/tasks/active/OTERYN-20261013-optional-ui-skills.md
proven:
  - Sources are pinned and licenses retained.
derived: []
unknown:
  - Native CLI auto-discovery is BLOCKED by authentication failure.
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
next_action: Qualify the final candidate; archive this packet only after terminal PR 1603 readback.
```

## Review and evidence

Independent AI review not selected: optional-aid instructions narrow selection without changing autonomous write/integration authority, runtime authorization or workflow gates. Self-review and deterministic checks required. The catalog distinguishes static validation from unperformed client/model trials.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Ordinary same-repository delivery; verify deletion after merge.
source_branch_evidence: Live PR 1603; terminal readback remains pending.
```

## Validation and terminal disposition

- Checkpoint and Documentation/Agent IA validation: PASS.
- `OTERYN_META_POLICY_ROOT=/workspace/oteryn-meta-policy python -m unittest discover -s tools/agents -p 'test_*.py' -q`: PASS, 158 tests.
- `python tools/validation/prompt_eval.py`: PASS, deterministic contract only; model trials 0.
- `python tools/agents/policy_consistency.py --meta-root /workspace/oteryn-meta-policy`: PASS against authenticated bound META revision.
- Source SHA-256/license checks and `git diff --check`: PASS.
- Changed-file self-review: PASS.

PR: https://github.com/Oteryn/Oteryn-Platform/pull/1603 . This packet remains active through qualification. Once PR #1603 is verified terminal, archive it with the terminal evidence in a bounded closeout change. The explicit archive-pending disposition preserves ownership during the terminal transition. Exact final SHA, CI, queue and terminal merge evidence are maintained in the live PR. Source deletion and ownership release require terminal readback. Runtime E2E is NOT_APPLICABLE because only optional agent aids are added. Explicit bootstrap/catalog model qualification passed: paired baseline/candidate trials, 12 decision cases each, candidate 12/12 intended selections. Native CLI auto-discovery remains BLOCKED; no universal enforcement claim.

## Bounded CI repair

The initial exact-head Agent Governance run 38442836688 found stale active ownership for already-merged PR #1591. This task also moves only `OTERYN-20261013-platform-pg-p6a-impl.md` from active to archive with verified merge, gate and branch-deletion evidence. No open PR #1598/#1599/#1601 owned that record at the allocation check; no P6 runtime or deploy implementation is modified. Live task liveness is rerun before the repaired head is frozen.

Live task liveness after reconciliation: PASS, 12 active tasks and 0 advisory findings. Both archived checkpoints and the same 158 agent-tool tests, documentation IA and deterministic prompt contracts passed after repair.

## Automatic review repairs

- Kept this packet active until verified terminal PR readback; recorded the explicit archive-pending transition.
- Removed the unmodified upstream server helper because port-only readiness cannot establish process ownership. No fork or new launcher is introduced.
- Normalized trailing whitespace in the upstream browser-testing reference and recorded separate source/local SHA-256 hashes; exact candidate whitespace validation is required, not an unstaged diff on a clean checkout.

Model qualification is recorded in .agents/skills/evaluation.json with the exact root/catalog/entry-point hashes consumed by the trial. Actual model IDs, token/time telemetry and native auto-discovery are not inferred.
