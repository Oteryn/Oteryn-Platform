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
updated_at: 2026-08-12T11:43:13+02:00
head: b027ffe1ff9d78173e01f9e31f9f0cc4287426bb
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
  - Fresh exact-head review on 14677cd534ceb204a78828285acc68238a502c71 reproduced a foreign-repository authorization bypass through unrelated local-file write approval, two completion-weakening phrasings that escaped detection, and a case-sensitive false positive for the canonical GitHub repository identity.
  - Material repair b027ffe1ff9d78173e01f9e31f9f0cc4287426bb binds active mutation authorization to the target repository or an explicit repository pronoun, compares the canonical repository identity case-insensitively, and rejects completion before exact-head CI passes or despite unresolved material findings.
  - Local validation on b027ffe1ff9d78173e01f9e31f9f0cc4287426bb passed 89 of 89 policy-consistency regressions, the live validator, 12 of 12 CI routing tests and git diff checks.
derived:
  - All currently known material policy-consistency findings have implementation repairs and focused regressions on b027ffe1ff9d78173e01f9e31f9f0cc4287426bb.
  - This checkpoint-only refresh does not change parser, routing, workflow or application behavior; b027ffe1ff9d78173e01f9e31f9f0cc4287426bb remains the latest material implementation/test head and live GitHub state is authoritative for the final metadata head created by this update.
unknown:
  - Terminal repository-required CI result on the final checkpoint head created by this update.
  - Fresh Codex review result for the final checkpoint head created by this update.
conflicts: []
first_failure:
  marker: codex-final-parser-gap-generation
  evidence: Fresh review on 14677cd534ceb204a78828285acc68238a502c71 reproduced unrelated-write authorization scope, two completion-weakening bypasses and canonical repository case sensitivity. Material repair b027ffe1ff9d78173e01f9e31f9f0cc4287426bb passes 89/89 focused regressions, the live validator and 12/12 routing tests locally.
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
  - command: python tools/agents/test_policy_consistency.py && python tools/agents/policy_consistency.py && python tests/ci/test_classify_changes.py on b027ffe1ff9d78173e01f9e31f9f0cc4287426bb
    result: PASS
    evidence: 89 of 89 policy-consistency regressions, live policy validation and 12 of 12 CI routing tests completed successfully; git diff checks also passed.
  - command: Agent Governance run 31578139727 on material head 2a68f1cb26a93f0ce4bc14e1aaab21e2d77983c5
    result: PASS
    evidence: job 94054827809 completed 82 of 82 policy-consistency regressions, live policy validation, 3 checkpoint validations, live liveness validation for 3 active tasks with 0 advisory findings and Control Room validation successfully.
  - command: material-head repository workflow generation
    result: FAIL
    evidence: CI 31578139670, Agent Governance 31578139727, Edge Security Emulation 31578139660, Platform DB Outage Validation 31578139657 and Game Auth Ticket Concurrency 31578139671 reached success; Deep System Validation 31578139668 remained active and Phase 7 31578139662 encountered an external composer TLS failure before application validation and was retried on the same head.
  - command: final metadata-head required CI and fresh Codex review
    result: NOT_RUN
    evidence: this task-record-only refresh creates the final validation generation.
blockers:
  - none
next_action: Commit and push this checkpoint, resolve the now-addressed review thread with exact repair evidence, then validate the resulting exact head with repository-required CI, Agent Governance and a fresh Codex review.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 25
  session_id: codex-20260812-1143-final-policy-parser-repair
  session_started_at: 2026-08-12T11:31:00+02:00
  checkpointed_at: 2026-08-12T11:43:13+02:00
  last_progress_at: 2026-08-12T11:43:13+02:00
  phase: final-checkpoint-validation
  exact_head: b027ffe1ff9d78173e01f9e31f9f0cc4287426bb
  pull_request: 992
  active_operation: checkpoint final parser-gap repairs after 89-test local proof
  external_run_ids: []
  operation_started_at: 2026-08-12T11:43:13+02:00
  wait_deadline_at: 2026-08-12T12:28:13+02:00
  check_generation: final-parser-gap-repair-checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: final checkpoint head required checks and fresh review are terminal and branch head remains unchanged
  next_action: push the final checkpoint, resolve the addressed thread with evidence, and inspect exact-head CI plus fresh review
```

## Notes

`feature_scope: internal_only`; user-facing/runtime E2E is `NOT_APPLICABLE` because this task changes repository governance, validation and routing only. Executable outcome proof is the focused Python regression suite, live policy validator, routing tests and repository-required exact-head checks. Generic workflow and classifier changes remain fail closed with all applicable gates.
