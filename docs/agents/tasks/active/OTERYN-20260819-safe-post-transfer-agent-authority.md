---
task_id: OTERYN-20260819-safe-post-transfer-agent-authority
status: validating
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
policy_version: 2
phase: validate
session_id: chatgpt-20260820-safe-post-transfer-authority
session_role: implementer-validator
execution_mode: github-actions-bounded-patch
execution_reason: preserve large fail-closed governance files byte-for-byte except exact asserted coordinate/path migrations
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: mechanically migrate authority, validate full fail-closed suite, then perform exact-head self-review and protected merge
validation_level: full
heavy_validation_runs: 1
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
issue: 1165
pr: 1178
branch: governance/issue-1165-safe-authority
base_sha: 3f5c86c17c704dad71cbd89b14dace155392ea10
owned_paths:
  - .github/CODEOWNERS
  - .github/workflows/agent-governance.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/phase7-production-like-validation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - AGENTS.md
  - README.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md
  - docs/agents/tasks/active/OTERYN-20260819-safe-post-transfer-agent-authority.md
  - scripts/ci/classify_changes.py
  - tests/ci/test_workflow_trigger_economy.py
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
---

# Safe post-transfer agent authority canonicalization

## Objective

Replace unsafe #1166 with a fail-closed migration that preserves the full existing governance parser/test matrix and bootstrap rules while making `Oteryn/Oteryn-Platform` the only current Platform write coordinate and removing authority semantics from the obsolete root same-directory override model.

## Guardrails

- no compression or deletion of governance semantics;
- the existing `policy_consistency.py` and `test_policy_consistency.py` logic is transformed only by exact asserted repository-identity/bootstrap-path replacements;
- the full former root override body is preserved in `docs/agents/PLATFORM_AGENT_BOOTSTRAP.md` with only the post-transfer repository-coordinate substitution plus a provenance header;
- root `AGENTS.md` explicitly requires the durable bootstrap in instruction order and lean startup;
- current workflow/classifier/prompt/task references route to the durable bootstrap; historical archive/evidence provenance is not rewritten;
- no architecture, contracts, native-protocol, runtime or deployment behavior changes;
- all temporary migration/update workflows must be absent before readiness;
- canonical repository policy `repair_delivery_model: one_issue_one_owner_self_review` applies: exact-head self-review is mandatory, while a second repair auditor is not a per-repair merge requirement;
- exact-head required CI, E2E classification, review hygiene and protected merge remain mandatory.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-20T07:34:00Z
head: 385195b9d69581e8cbf73e026a0832dc78ef23aa
branch: governance/issue-1165-safe-authority
pr: 1178
status: validating
context_routes:
  - agent-governance
  - testing
  - repository-migration
owned_paths:
  - .github/CODEOWNERS
  - .github/workflows/agent-governance.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/phase7-production-like-validation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - AGENTS.md
  - README.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md
  - docs/agents/tasks/active/OTERYN-20260819-safe-post-transfer-agent-authority.md
  - scripts/ci/classify_changes.py
  - tests/ci/test_workflow_trigger_economy.py
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
proven:
  - PR 1166 has two independent HIGH findings and is not merge authority.
  - Current canonical repository identity is Oteryn/Oteryn-Platform.
  - Provider physical transfer PR 1164 merged as a621a94d727be35ab73afe7d59f0e182cfd61356 and lifecycle closeout PR 1179 merged as 3f5c86c17c704dad71cbd89b14dace155392ea10.
  - The complete former root override policy body is preserved in docs/agents/PLATFORM_AGENT_BOOTSTRAP.md except for the exact post-transfer repository-coordinate substitution and provenance header.
  - Root AGENTS.md explicitly requires the durable bootstrap before substantial work.
  - The full fail-closed policy-consistency suite ran 95 tests successfully after the mechanical migration.
  - policy_consistency.py passed on the migrated tree.
  - workflow trigger economy, checkpoint, task-liveness and prompt-contract regressions passed during bounded migration validation.
  - Five workflow files were generated by exact asserted bootstrap-path substitution, validated together with the full governance suite, and delivered as one workflow-only Git commit object before authorized fast-forward.
  - All temporary authority-migration and main-update workflows are absent from the current delivery tree.
  - Branch was merged with protected main 3f5c86c17c704dad71cbd89b14dace155392ea10 without force and without conflict.
derived:
  - The two HIGH findings from unsafe PR 1166 are addressed by preservation rather than compression of governance semantics and by explicit durable bootstrap routing.
  - No separate repair auditor is required because trusted base policy explicitly selects one_issue_one_owner_self_review and external_repair_auditor_required false.
  - Runtime E2E is NOT_APPLICABLE because this change alters governance/document/workflow routing only and performs no product/runtime/deployment behavior change.
unknown: []
conflicts: []
first_failure:
  marker: final-governance-metadata
  evidence: pre-closeout Agent Governance showed policy tests PASS but required PR identity and changed_paths in the task checkpoint; terminal PR 1164 also had to be archived first.
rejected_hypotheses:
  - A shortened replacement validator is equivalent to the existing fail-closed parser.
  - Removing the root override without preserving and mandatorily loading its rules is safe.
  - A second repair auditor is mandatory despite the trusted base explicitly disabling that requirement.
changed_paths:
  - .github/CODEOWNERS
  - .github/workflows/agent-governance.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/phase7-production-like-validation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - AGENTS.md
  - README.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md
  - docs/agents/tasks/active/OTERYN-20260819-safe-post-transfer-agent-authority.md
  - scripts/ci/classify_changes.py
  - tests/ci/test_workflow_trigger_economy.py
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
validation:
  - command: python tools/agents/test_policy_consistency.py
    result: PASS
    evidence: 95 of 95 fail-closed regression tests passed on the mechanically migrated tree
  - command: python tools/agents/policy_consistency.py
    result: PASS
    evidence: migrated current repository authority and bootstrap routing are internally consistent
  - command: python tests/ci/test_workflow_trigger_economy.py
    result: PASS
    evidence: workflow path routing remains economical and supersedable workflows retain scoped concurrency
  - command: python tools/agents/test_checkpoint.py and python tools/agents/test_task_liveness.py
    result: PASS
    evidence: bounded migration validation passed checkpoint and liveness unit suites; exact live PR validation is re-running on this corrected checkpoint
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: governance, documentation and CI routing only; no product/runtime/deployment behavior is changed
blockers: []
next_action: Run fresh exact-head Agent Governance and required CI on the corrected up-to-date branch, perform full diff self-review and review-thread hygiene, then mark Ready and squash-merge only if every required gate passes.
```
