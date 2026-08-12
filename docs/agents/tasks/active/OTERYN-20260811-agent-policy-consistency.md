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
updated_at: 2026-08-12T09:31:00+02:00
head: 1c04f6328730c848231c1b54c54bb8df46b8a64e
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
  - Earlier review generations exposed and drove regression coverage for repository-scope grants, authorization scoping and polarity, Markdown wrapping/emphasis/fences, slash-shaped prose, active/passive/mandatory mutation grammar, duplicate declarations, nested YAML examples and checkpoint freshness.
  - Fresh Codex review on checkpoint head 3dc0c3429f8b96c913109b1dabfd494d0bbe23ed exposed two additional material findings: have/has write access to a foreign repository was not recognized as an affirmative mutation grant, and may refrain from editing a foreign repository was incorrectly treated as a positive mutation grant.
  - Parser commit eaeab0c4ad59228ffcab30db09527194bf71e1fe recognizes have/has write access as positive mutation authority and treats refrain from a mutation as clause-local denial/non-grant semantics.
  - Test head 1c04f6328730c848231c1b54c54bb8df46b8a64e adds focused regressions for have write access, has write access, refrain-from denial, and a later independent affirmative grant after a refrain clause.
  - Compare 3dc0c3429f8b96c913109b1dabfd494d0bbe23ed...1c04f6328730c848231c1b54c54bb8df46b8a64e changes exactly tools/agents/policy_consistency.py (+5/-2) and tools/agents/test_policy_consistency.py (+22), with no unrelated path change.
  - Agent Governance run 31574092122 passed on 1c04f6328730c848231c1b54c54bb8df46b8a64e. Job 94042199953 executed 69 of 69 policy-consistency regressions successfully, including all four new cases, then passed the live policy validator, 3 active checkpoint validations, live liveness validation for 3 active tasks with 0 advisory findings, and Control Room validation.
derived:
  - All currently known material parser findings have implementation repairs and explicit focused regressions; this task-record-only refresh creates a new final checkpoint generation without changing parser/runtime behavior.
  - Recording 1c04f6328730c848231c1b54c54bb8df46b8a64e as the latest material implementation/test head avoids self-referential checkpoint SHA churn; live GitHub state remains authoritative for the actual branch head produced by this checkpoint commit.
unknown:
  - Terminal repository-required CI result on the checkpoint-only final head created by this update.
  - Fresh Codex review result for the checkpoint-only final head created by this update.
conflicts: []
first_failure:
  marker: codex-write-access-and-refrain-polarity
  evidence: Fresh Codex review on 3dc0c3429f8b96c913109b1dabfd494d0bbe23ed identified have/has write access as a missed foreign-repository grant and may refrain from editing as a false-positive grant; material head 1c04f6328730c848231c1b54c54bb8df46b8a64e repairs both and Agent Governance 31574092122 proves 69 focused regressions plus the live validator pass.
rejected_hypotheses:
  - Existing permission/may/can/allow vocabulary covers every affirmative write grant; direct have/has write access wording can also expand the effective write boundary.
  - A modal may anywhere near a mutation makes the clause affirmative; refrain from editing is denial/non-grant language and must not be promoted to mutation authority.
  - Treating refrain as a denial can suppress later grants in the same logical statement; regression coverage proves the denial remains clause-local while a later independent affirmative grant is still rejected.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: Agent Governance run 31574092122 on implementation/test head 1c04f6328730c848231c1b54c54bb8df46b8a64e
    result: PASS
    evidence: job 94042199953 completed 69 of 69 policy-consistency regressions, including the four final review regressions, live policy validation, 3 checkpoint validations, live liveness validation for 3 active tasks with 0 advisory findings and Control Room validation successfully.
  - command: minimal final-review repair comparison
    result: PASS
    evidence: compare 3dc0c3429f8b96c913109b1dabfd494d0bbe23ed...1c04f6328730c848231c1b54c54bb8df46b8a64e contains exactly two intended files with +5/-2 parser and +22 test changes.
  - command: checkpoint-only final-head required CI, Agent Governance and fresh Codex review
    result: NOT_RUN
    evidence: this task-record-only update creates the final validation generation.
blockers:
  - none
next_action: Validate the checkpoint-only final branch head created by this update with repository-required CI, Agent Governance and a fresh Codex review; if every required check passes with zero unresolved material threads, squash-merge PR #992 with expected-head protection, verify main, close Issue #991 and archive this task through the repository-mandated lifecycle.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 22
  session_id: agent-20260812-0931-final-grant-parser-checkpoint
  session_started_at: 2026-08-12T09:31:00+02:00
  checkpointed_at: 2026-08-12T09:31:00+02:00
  last_progress_at: 2026-08-12T09:31:00+02:00
  phase: final-checkpoint-validation
  exact_head: 1c04f6328730c848231c1b54c54bb8df46b8a64e
  pull_request: 992
  active_operation: create refreshed final checkpoint after write-access and refrain-polarity repairs and 69-test Agent Governance proof
  external_run_ids:
    - 31574092122
    - 31574092149
    - 31574092118
    - 31574092124
    - 31574092125
    - 31574092150
    - 31574092175
  operation_started_at: 2026-08-12T09:31:00+02:00
  wait_deadline_at: 2026-08-12T10:16:00+02:00
  check_generation: write-access-refrain-final-checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: checkpoint-only exact-head required checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect final exact-head CI and fresh review, then squash-merge only if every merge gate remains satisfied
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` because this task changes repository governance, validation and routing only. Executable outcome proof is the focused Python regression suite, live policy validator, routing tests and repository-required exact-head checks. Generic workflow and classifier changes remain fail closed with all applicable gates.