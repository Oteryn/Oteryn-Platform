---
task_id: OTERYN-20260819-safe-post-transfer-agent-authority
status: implementing
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
policy_version: 2
phase: implement
session_id: chatgpt-20260819-safe-post-transfer-authority
session_role: implementer
execution_mode: github-actions-bounded-patch
execution_reason: preserve large fail-closed governance files byte-for-byte except exact asserted coordinate/path migrations
context_pressure: high
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: first migrate instruction discovery and validator paths mechanically, then validate and reconcile the superseded unsafe PR
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
issue: 1165
branch: governance/issue-1165-safe-authority
base_sha: 256f27ba97f4b103320c186211583ea7c13dcf33
owned_paths:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
  - scripts/ci/classify_changes.py
  - .github/workflows/agent-governance.yml
  - .github/workflows/one-off-governance-authority-migration.yml
  - .github/CODEOWNERS
  - SECURITY.md
  - .github/dependabot.yml
  - .gitignore
  - docs/agents/tasks/active/OTERYN-20260819-safe-post-transfer-agent-authority.md
---

# Safe post-transfer agent authority canonicalization

## Objective

Replace the unsafe #1166 approach with a fail-closed migration that preserves the full existing governance parser/test matrix and bootstrap rules while making `Oteryn/Oteryn-Platform` the only current Platform write coordinate and removing the obsolete same-directory root override discovery model.

## Guardrails

- no compression/deletion of governance semantics;
- the existing `policy_consistency.py` and `test_policy_consistency.py` logic is transformed only by exact asserted token/path replacements;
- the full former root override body is moved intact to `docs/agents/PLATFORM_AGENT_BOOTSTRAP.md` and remains a mandatory root read;
- no architecture/contracts/native-protocol edits; those remain #1172 scope;
- temporary patch workflow is branch-only and must be deleted before Ready/merge;
- exact-head governance/CI and fresh independent review required before merge.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-19T23:35:00+02:00
head: 256f27ba97f4b103320c186211583ea7c13dcf33
branch: governance/issue-1165-safe-authority
pr: null
status: implementing
context_routes:
  - agent-governance
  - testing
owned_paths:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
  - scripts/ci/classify_changes.py
  - .github/workflows/agent-governance.yml
  - .github/workflows/one-off-governance-authority-migration.yml
  - .github/CODEOWNERS
  - SECURITY.md
  - .github/dependabot.yml
  - .gitignore
  - docs/agents/tasks/active/OTERYN-20260819-safe-post-transfer-agent-authority.md
proven:
  - PR 1166 exact head has two independent HIGH findings and must not merge.
  - Current main retains the full fail-closed parser/test matrix and strict root bootstrap.
  - Current canonical repository identity is Oteryn/Oteryn-Platform.
derived:
  - A mechanical asserted transformation can migrate identity/discovery without common-mode manual truncation.
unknown: []
conflicts: []
first_failure:
  marker: unsafe-governance-compression
  evidence: independent review 4976994598 on PR 1166
rejected_hypotheses:
  - A shortened replacement validator is equivalent to the existing fail-closed parser.
  - Removing the root override without preserving/loading its rules is safe.
validation:
  - command: exact-head CI
    result: NOT_RUN
    evidence: implementation generation pending
blockers: []
next_action: Run the branch-only asserted migration workflow, delete it, inspect the exact diff, then run full exact-head governance validation.
```
