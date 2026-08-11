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
updated_at: 2026-08-12T00:40:00+02:00
head: 9d991c3c96af8a57429618fd5ced48034f46967a
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
  - Earlier review generations exposed and drove regression coverage for repository-scope grants, authorization scoping and polarity, Markdown wrapping/emphasis/fences, slash-shaped prose, active/passive mutation grammar, duplicate declarations, nested YAML examples and checkpoint freshness.
  - Fresh Codex review on checkpoint head e40225bfc959002a618f352078a4e52cc22c011d exposed two additional P1 findings: mandatory mutation grants such as must/shall edit were not recognized, and independent passive/permission grants after conjunctions were not split from an earlier conditional authorization clause.
  - Source commit 6437571cd44ed9801853dce5c6656ac8493d0f6f recognizes mutation-specific must/shall/required-to forms without treating bare mandatory words as universal authorization, and recognizes passive allowed/authorized/permitted/required plus permission-to mutation clauses as independent grants.
  - Test head 9d991c3c96af8a57429618fd5ced48034f46967a adds regressions for must, shall and required-to mutation grants; passive allowed and permission grants after conjunctions; and the negative must-not-edit case while retaining all prior regressions.
  - Agent Governance run 31543265797 passed on 9d991c3c96af8a57429618fd5ced48034f46967a. Job 93950188307 executed 65 of 65 policy-consistency regressions successfully, then passed the live policy validator, 3 active checkpoint validations, live liveness validation for 3 active tasks with 0 advisory findings, and Control Room validation.
  - The two P1 review threads from e40225bfc959002a618f352078a4e52cc22c011d are resolved only after the 65-test exact-head Agent Governance evidence on 9d991c3c96af8a57429618fd5ced48034f46967a.
derived:
  - All currently known material parser findings have implementation repairs and explicit focused regressions; this task-record-only refresh creates a new final checkpoint generation without changing parser/runtime behavior.
  - Recording 9d991c3c96af8a57429618fd5ced48034f46967a as the latest material implementation/test head avoids self-referential checkpoint SHA churn; live GitHub state remains authoritative for the actual branch head produced by this checkpoint commit.
unknown:
  - Terminal repository-required CI result on the checkpoint-only final head created by this update.
  - Fresh Codex review result for the checkpoint-only final head created by this update.
conflicts: []
first_failure:
  marker: codex-mandatory-and-passive-grant-bypasses
  evidence: Fresh Codex review on e40225bfc959002a618f352078a4e52cc22c011d identified mandatory must/shall mutation grants and conjunction-introduced passive grants as P1 bypasses; implementation/test head 9d991c3c96af8a57429618fd5ced48034f46967a repairs both and Agent Governance 31543265797 proves 65 focused regressions plus the live validator pass.
rejected_hypotheses:
  - Positive authorization vocabulary limited to may/can/allow/permission is sufficient; mandatory must/shall/required-to mutation rules can also expand the effective write boundary and must be rejected for unauthorized repositories.
  - Independent grants after and always begin with a direct mutation or may/can; passive are allowed/authorized/permitted/required to and have permission to forms must be split before applying a left-side authorization exception.
  - Adding must globally to positive authorization is safe; mandatory authority is recognized only when syntactically bound to a mutation, preserving must-not and unrelated mandatory prose.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: Agent Governance run 31543265797 on implementation/test head 9d991c3c96af8a57429618fd5ced48034f46967a
    result: PASS
    evidence: job 93950188307 completed 65 of 65 policy-consistency regressions, live policy validation, 3 checkpoint validations, live liveness validation for 3 active tasks with 0 advisory findings and Control Room validation successfully.
  - command: e40225b final-head repository-required CI and Agent Governance before the latest review repairs
    result: PASS
    evidence: CI 31542583723 and Agent Governance 31542583727 passed on e40225bfc959002a618f352078a4e52cc22c011d; that generation was not mergeable because its fresh Codex review subsequently produced two P1 findings that are now repaired on 9d991c3c96af8a57429618fd5ced48034f46967a.
  - command: checkpoint-only final-head required CI, Agent Governance and fresh Codex review
    result: NOT_RUN
    evidence: this task-record-only update creates the next exact-head validation generation.
blockers:
  - none
next_action: Validate the checkpoint-only final branch head created by this update with repository-required CI, Agent Governance and a fresh Codex review; if every required check passes with zero unresolved material threads, squash-merge PR #992 with expected-head protection, verify main, close Issue #991 and archive this task through the repository-mandated lifecycle.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 21
  session_id: agent-20260812-0040-final-checkpoint-refresh
  session_started_at: 2026-08-12T00:40:00+02:00
  checkpointed_at: 2026-08-12T00:40:00+02:00
  last_progress_at: 2026-08-12T00:40:00+02:00
  phase: final-checkpoint-validation
  exact_head: 9d991c3c96af8a57429618fd5ced48034f46967a
  pull_request: 992
  active_operation: create refreshed final checkpoint after mandatory/passive grant repairs and 65-test Agent Governance proof
  external_run_ids:
    - 31543265797
    - 31543265772
    - 31543265751
    - 31543265786
    - 31543265768
    - 31543265750
    - 31543265784
  operation_started_at: 2026-08-12T00:40:00+02:00
  wait_deadline_at: 2026-08-12T01:25:00+02:00
  check_generation: mandatory-passive-final-checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: checkpoint-only exact-head required checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect final exact-head CI and fresh review, then squash-merge only if every merge gate remains satisfied
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` because this task changes repository governance, validation and routing only. Executable outcome proof is the focused Python regression suite, live policy validator, routing tests and repository-required exact-head checks. Generic workflow and classifier changes remain fail closed with all applicable gates.
