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

- [x] Binding and independent workflow trust anchor equal exact protected META `33b212e652c680bd4047be3b414c9a358b8bf26f`.
- [x] Prepared-candidate publication failure preserves custody and forbids raw Git Data/per-file API reconstruction fallback.
- [x] `tools/agents/test_policy_consistency.py` and authenticated provider validator pass against the exact META checkout.
- [x] Exact-head required Platform CI and review are clean.
- [x] Integration uses governed/native Merge Queue, real merge-group `platform-gate`, and protected-main readback.

## Ownership

```yaml
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/archive/OTERYN-20260917-publication-integrity-provider-adoption.md
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
updated_at: 2026-09-25T08:50:00Z
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
  - docs/agents/tasks/archive/OTERYN-20260917-publication-integrity-provider-adoption.md
proven:
  - protected META publication-integrity authority at original adoption was 33b212e652c680bd4047be3b414c9a358b8bf26f
  - Platform main at task creation was 84d504c98acc8134eb4c9545711010b74c987974
  - PR #1402 exact head 27aa5023ddd36d783f6c8020e60e99d91d5c2f16 passed required exact-head Platform qualification and independent review
  - PR #1402 merged through the governed Platform integration path as 623435ec1b907d6d9770b767806c90300252a71c
  - successor PR #1412 established protected META authority 1bfb5ff98c8aa156e73669a14e083a1d464c29fb
  - terminal closeout PR #1413 merged through governed Merge Queue as protected main 6538a138321d08262ee034d896abc86ee110d9de
  - governing Issue #1401 is closed after protected-main terminal readback
derived:
  - independent workflow trust anchor must move with META_AGENT_POLICY_BINDING.json
unknown: []
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
  - docs/agents/tasks/archive/OTERYN-20260917-publication-integrity-provider-adoption.md
validation:
  - command: exact-head PR #1402 required Platform qualification
    result: PASS
    evidence: exact head 27aa5023ddd36d783f6c8020e60e99d91d5c2f16; platform-gate and CodeQL succeeded and independent Codex review reported no major issues
  - command: terminal protected-main lifecycle readback
    result: PASS
    evidence: Issue #1401 terminal comment records PR #1413 Merge Queue integration as 6538a138321d08262ee034d896abc86ee110d9de with merge-group platform-gate success
blockers:
  - none
next_action: archive complete; no active ownership remains for this packet
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository governance adoption PR
source_branch_evidence: live GitHub readback returned 404 Branch not found for governance/publication-integrity-adoption-1401 after PR #1402 merged
```

## Notes

No product/runtime/auth/payment/database/deployment/production/secret/ruleset/protection mutation. No new publication framework; local wording only mirrors the candidate-preservation boundary required by bound META.
