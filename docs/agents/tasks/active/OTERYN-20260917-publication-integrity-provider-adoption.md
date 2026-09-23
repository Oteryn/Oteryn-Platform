---
task_id: OTERYN-20260917-publication-integrity-provider-adoption
governing_issue: 1401
required_reads:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
search_first:
  - publication integrity
optional_reads: []
---

# OTERYN-20260917-publication-integrity-provider-adoption

## Goal

Governing GitHub Issue: #1401 — canonical lifecycle authority for this task.

Adopt protected META publication-integrity authority `33b212e652c680bd4047be3b414c9a358b8bf26f` with the smallest Platform-local consumer change: exact binding/trust-anchor repin, one fail-closed bootstrap sentence, and representative policy-consumption regression coverage.

## Acceptance criteria

- [ ] Binding and independent workflow trust anchor equal exact protected META `33b212e652c680bd4047be3b414c9a358b8bf26f`.
- [ ] Prepared-candidate publication failure preserves custody and forbids raw Git Data/per-file API reconstruction fallback.
- [ ] `tools/agents/test_policy_consistency.py` and authenticated provider validator pass against the exact META checkout.
- [ ] Exact-head required Platform CI and review are clean.
- [ ] Integration uses governed/native Merge Queue, real merge-group `platform-gate`, and protected-main readback.

## Ownership

```yaml
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/active/OTERYN-20260917-publication-integrity-provider-adoption.md
modules:
  - agent-governance
dependencies:
  - Oteryn/Oteryn#212
  - Oteryn/Oteryn#213
blockers:
  - none
cross_repository_tasks:
  - PUBLICATION-INTEGRITY-PROVIDER-ROLLOUT
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-23T18:34:00Z
head: 27aa5023ddd36d783f6c8020e60e99d91d5c2f16
branch: governance/publication-integrity-adoption-1401
pr: 1402
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - governance
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/active/OTERYN-20260917-publication-integrity-provider-adoption.md
proven:
  - protected META publication-integrity authority is 33b212e652c680bd4047be3b414c9a358b8bf26f
  - Platform main at task creation is 84d504c98acc8134eb4c9545711010b74c987974
  - Draft PR #1402 owns this exact provider-adoption lineage
derived:
  - independent workflow trust anchor must move with META_AGENT_POLICY_BINDING.json
unknown:
  - terminal exact-head CI result after this lifecycle-metadata correction
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/active/OTERYN-20260917-publication-integrity-provider-adoption.md
validation:
  - command: Agent Governance / policy consistency
    result: NOT_RUN
    evidence: fresh exact-head qualification is required after the lifecycle-metadata correction
blockers:
  - none
next_action: archive this completed provider-adoption packet after protected-main lifecycle readback; keep any later Issue #1401 work in a fresh bounded task
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository governance adoption PR
source_branch_evidence: pending protected integration
```

## Notes

No product/runtime/auth/payment/database/deployment/production/secret/ruleset/protection mutation. No new publication framework; local wording only mirrors the candidate-preservation boundary required by bound META.
