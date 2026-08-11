---
task_id: OTERYN-20260811-agent-policy-consistency
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
search_first:
  - agent governance consistency
  - open PR ownership for AGENTS and tools/agents
optional_reads: []
---

# OTERYN-20260811-agent-policy-consistency

## Goal

Fail closed when duplicated agent-governance status, execution-budget, repository-scope or completion rules drift across canonical Oteryn Platform policy documents.

Tracks Issue #991.

## Acceptance criteria

- [x] Deterministic validator covers current trusted governance and contradictory scope declarations.
- [x] Focused negative fixtures prove task-status, budget, repository-scope and closeout drift are rejected.
- [x] Agent Governance CI executes the focused tests and live validator.
- [x] Governance-only changes, including directory-based PR templates, route without unrelated heavy runtime gates.
- [ ] Exact-head required CI, Agent Governance and fresh review pass before merge.
- [ ] Issue #991 closes and this task archives after merge.

## Ownership

```yaml
owned_paths:
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
  - .github/workflows/agent-governance.yml
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - docs/agents/tasks/archive/OTERYN-20260811-agent-policy-consistency.md
modules:
  - agent-governance
  - testing
  - ci-routing
dependencies:
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-12T00:28:00+02:00
head: e0deeb72e5450179c57b065b13d905b76af83506
branch: test/agent-policy-consistency
pr: 992
status: validating
context_routes:
  - agent-governance
  - testing
  - ci-repair
owned_paths:
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
  - .github/workflows/agent-governance.yml
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - docs/agents/tasks/archive/OTERYN-20260811-agent-policy-consistency.md
proven:
  - Protected main ab43c4b47173e7208d34851c4091f79051379f7a is incorporated and the PR remains limited to the six declared task-owned paths.
  - Fresh Codex review on 530b4c6b8f3d4870e135e21a694d9db8a5748394 exposed three material findings: authorization exceptions were not bound to the actual write target, canonical YAML extraction admitted nested fenced examples, and this task checkpoint still described the older 430f338 implementation generation.
  - Implementation head e0deeb72e5450179c57b065b13d905b76af83506 binds authorization exceptions to the relevant repository and mutation scope, rejects read-only or other-repository authorization as a write exception, and parses canonical anti-stall YAML only from top-level yaml fences.
  - Focused regressions on e0deeb72e5450179c57b065b13d905b76af83506 cover nested YAML status and budget examples, read authorization not unlocking writes, authorization concerning another repository not unlocking the target, matching write authorization, and all previously repaired governance-policy variants.
  - Agent Governance run 31542372168 passed on implementation head e0deeb72e5450179c57b065b13d905b76af83506 with 59 of 59 policy-consistency regressions, the live policy validator, checkpoint validation, live task liveness and Control Room validation all successful.
  - Edge Security Emulation 31542372187 and Game Auth Ticket Concurrency 31542372160 passed on implementation head e0deeb72e5450179c57b065b13d905b76af83506.
  - The authorization-target and nested-YAML review threads from the 530b4c6 review are resolved after the exact-head Agent Governance proof; the checkpoint-freshness thread remains open until this task-record update is committed.
derived:
  - All currently known parser findings have implementation repairs and focused regression coverage; this task-record-only update creates a fresh final checkpoint generation without changing runtime or parser behavior.
  - The checkpoint records the latest material implementation head rather than attempting to record the SHA of the commit that contains the checkpoint itself; live GitHub state remains authoritative for the actual branch head.
unknown:
  - Terminal repository-required check result on the checkpoint-only final head created by this update.
  - Fresh Codex review result for the checkpoint-only final head created by this update.
conflicts: []
first_failure:
  marker: codex-authorization-target-nested-yaml-and-stale-checkpoint
  evidence: Fresh Codex review on 530b4c6b8f3d4870e135e21a694d9db8a5748394 identified the three material findings; implementation head e0deeb72e5450179c57b065b13d905b76af83506 repairs the two code findings and Agent Governance 31542372168 proves 59 focused regressions plus the live validator pass, while this update repairs the checkpoint finding.
rejected_hypotheses:
  - Any nearby affirmative authorization can exempt a foreign-repository write grant; the authorization must apply to the actual write target and cannot be merely read-only or for another repository.
  - Any YAML-looking block in the anti-stall document is authoritative; only top-level yaml fences are parsed as canonical structured declarations.
  - A task-record checkpoint can safely remain on the older 430f338 generation after later material parser repairs; continuation evidence must be refreshed after those repairs.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: Agent Governance run 31542372168 on implementation head e0deeb72e5450179c57b065b13d905b76af83506
    result: PASS
    evidence: 59 of 59 policy-consistency regressions, live policy validator, checkpoint validation, live task liveness and Control Room validation completed successfully.
  - command: completed focused repository workflows on implementation head e0deeb72e5450179c57b065b13d905b76af83506
    result: PASS
    evidence: Edge Security Emulation 31542372187 and Game Auth Ticket Concurrency 31542372160 completed successfully; main CI 31542372161 was still in progress at checkpoint time and no terminal result is claimed here.
  - command: final checkpoint-head required CI, Agent Governance and fresh Codex review
    result: NOT_RUN
    evidence: this task-record-only update creates the final validation generation and therefore requires new exact-head evidence.
blockers:
  - none
next_action: Validate the checkpoint-only final branch head with repository-required CI, Agent Governance and a fresh Codex review; if every required check passes with zero unresolved material threads, squash-merge PR #992 with expected-head protection, verify main, close Issue #991 and archive this task through the repository-mandated lifecycle.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 20
  session_id: agent-20260812-0028-final-checkpoint-refresh
  session_started_at: 2026-08-12T00:28:00+02:00
  checkpointed_at: 2026-08-12T00:28:00+02:00
  last_progress_at: 2026-08-12T00:28:00+02:00
  phase: final-checkpoint-validation
  exact_head: e0deeb72e5450179c57b065b13d905b76af83506
  pull_request: 992
  active_operation: create refreshed final checkpoint after latest Codex-review repairs
  external_run_ids:
    - 31542372168
    - 31542372161
    - 31542372187
    - 31542372160
    - 31542372163
    - 31542372152
    - 31542372159
  operation_started_at: 2026-08-12T00:28:00+02:00
  wait_deadline_at: 2026-08-12T01:13:00+02:00
  check_generation: final-checkpoint-refresh
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: checkpoint-only exact-head required checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect final exact-head CI and fresh review, then squash-merge only if every merge gate remains satisfied
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` because this task changes repository governance, validation and routing only. Executable outcome proof is the focused Python regression suite, live policy validator, routing tests and repository-required exact-head checks. Generic workflow and classifier changes remain fail closed with all applicable gates.
