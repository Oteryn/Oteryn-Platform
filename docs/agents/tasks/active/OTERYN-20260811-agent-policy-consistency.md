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
- [x] Governance-only changes, including directory-based PR templates, route without unrelated heavy runtime gates by design.
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
updated_at: 2026-08-12T10:27:00+02:00
head: 2a68f1cb26a93f0ce4bc14e1aaab21e2d77983c5
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
  - Protected main ab43c4b47173e7208d34851c4091f79051379f7a was incorporated at the latest ownership/base check and the PR diff remained limited to the six declared task-owned paths.
  - Earlier Codex review generations drove explicit fail-closed coverage for repository grants, authorization scoping and polarity, Markdown wrapping/emphasis/fences, slash-shaped prose, active/passive/mandatory mutation grammar, duplicate declarations, nested YAML examples, task checkpoint freshness, write-access variants, destructive mutations and denial forms.
  - Exact-head review on 1efead13f342280aea6961ef301e9b6ff0453e6b exposed two final P1 findings: a positive foreign-repository mutation could be hidden by a same-clause read-only assertion, and completion requirements could be weakened by contradictory authoritative declarations while retaining the required marker text.
  - Parser repair commits e868bbd1caf37fb1f3356793986ca84cb5811ee9 and 7620306b37a744ce655e6d4e5420c66bf06fca13 prevent read-only exemption of positive mutation grants and add fenced-aware contradiction detection for completion requirements while narrowing the weakening grammar to avoid false positives.
  - Material head 2a68f1cb26a93f0ce4bc14e1aaab21e2d77983c5 adds regressions for positive grants paired with read-only assertions, optional exact-final-head CI, completion with unresolved review threads, completion without task archival and fenced non-authoritative weakening examples.
  - Agent Governance run 31578139727 passed on material head 2a68f1cb26a93f0ce4bc14e1aaab21e2d77983c5. Job 94054827809 executed 82 of 82 policy-consistency regressions successfully, then passed the live policy validator, 3 active checkpoint validations, live liveness for 3 active tasks with 0 advisory findings, and Control Room validation.
  - The two P1 threads from exact-head review 1efead13f342280aea6961ef301e9b6ff0453e6b were replied to with the 82-test proof and resolved only after that proof existed.
  - On material head 2a68f1cb26a93f0ce4bc14e1aaab21e2d77983c5, CI, Agent Governance, Edge Security Emulation, Platform DB Outage Validation and Game Auth Ticket Concurrency reached success before this checkpoint refresh.
  - Phase 7 run 31578139662 failed before application validation during composer dependency download with curl error 60 / self-signed certificate against GitHub API; no task-owned runtime/application code participates in that failure and the failed exact-head job was retried without changing the branch.
derived:
  - All currently known material policy-consistency findings have implementation repairs and focused regressions on 2a68f1cb26a93f0ce4bc14e1aaab21e2d77983c5.
  - This task-record-only refresh does not change parser, routing, workflow or application behavior; 2a68f1cb26a93f0ce4bc14e1aaab21e2d77983c5 remains the latest material implementation/test head and live GitHub state is authoritative for the final metadata head created by this update.
unknown:
  - Terminal repository-required CI result on the final metadata head created by this update.
  - Fresh Codex review result for the final metadata head created by this update.
conflicts: []
first_failure:
  marker: codex-read-only-and-completion-weakening-generation
  evidence: Fresh Codex review on 1efead13f342280aea6961ef301e9b6ff0453e6b identified read-only exemption of an affirmative mutation grant and marker-only completion validation. Material head 2a68f1cb26a93f0ce4bc14e1aaab21e2d77983c5 repairs both and Agent Governance 31578139727 / job 94054827809 proves 82/82 regressions plus live policy validation pass.
rejected_hypotheses:
  - A positive read-only assertion may safely exempt the rest of the same clause; an affirmative mutation grant in that clause must still fail closed.
  - Presence of closeout marker substrings alone proves the completion contract cannot be weakened; contradictory authoritative declarations must also be rejected.
  - Completion weakening detection may match any use of with/without near closeout terms; only explicit weakening grammar is rejected and fenced examples remain non-authoritative.
  - Phase 7 composer curl error 60 demonstrates an application regression; the job stopped at dependency download before application validation and the failure is external to the task-owned diff.
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260811-agent-policy-consistency.md
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: Agent Governance run 31578139727 on material head 2a68f1cb26a93f0ce4bc14e1aaab21e2d77983c5
    result: PASS
    evidence: job 94054827809 completed 82 of 82 policy-consistency regressions, live policy validation, 3 checkpoint validations, live liveness validation for 3 active tasks with 0 advisory findings and Control Room validation successfully.
  - command: material-head repository workflow generation
    result: PARTIAL
    evidence: CI 31578139670, Agent Governance 31578139727, Edge Security Emulation 31578139660, Platform DB Outage Validation 31578139657 and Game Auth Ticket Concurrency 31578139671 reached success; Deep System Validation 31578139668 remained active and Phase 7 31578139662 encountered an external composer TLS failure before application validation and was retried on the same head.
  - command: final metadata-head required CI and fresh Codex review
    result: NOT_RUN
    evidence: this task-record-only refresh creates the final validation generation.
blockers:
  - none
next_action: Validate the final metadata-only branch head created by this update with repository-required exact-head CI, Agent Governance and a fresh Codex review; if every required check passes with zero unresolved material threads and ownership/base state remains conflict-free, squash-merge PR #992 with expected-head protection, verify main, close Issue #991 and archive/release this task through the repository-mandated lifecycle.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 24
  session_id: agent-20260812-1027-final-policy-consistency-checkpoint
  session_started_at: 2026-08-12T10:13:00+02:00
  checkpointed_at: 2026-08-12T10:27:00+02:00
  last_progress_at: 2026-08-12T10:27:00+02:00
  phase: final-checkpoint-validation
  exact_head: 2a68f1cb26a93f0ce4bc14e1aaab21e2d77983c5
  pull_request: 992
  active_operation: create final checkpoint after read-only/completion contradiction repairs and 82-test Agent Governance proof
  external_run_ids:
    - 31578139727
    - 31578139670
    - 31578139668
    - 31578139662
    - 31578139660
    - 31578139671
    - 31578139657
  operation_started_at: 2026-08-12T10:27:00+02:00
  wait_deadline_at: 2026-08-12T11:12:00+02:00
  check_generation: final-read-only-completion-checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: final metadata-head required checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect final exact-head CI and fresh review, then squash-merge only if every merge gate remains satisfied
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` because this task changes repository governance, validation and routing only. Executable outcome proof is the focused Python regression suite, live policy validator, routing tests and repository-required exact-head checks. Generic workflow and classifier changes remain fail closed with all applicable gates.