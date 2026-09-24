---
task_id: OTERYN-20260924-meta-routing-authority-repin
governing_issue: 1401
required_reads:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
search_first:
  - integration capability routing
optional_reads: []
---

# OTERYN-20260924-meta-routing-authority-repin

## Goal

Governing GitHub Issue: #1401 — successor provider-adoption lane after protected META PR #224.

Repin Platform's immutable META 3.1 binding, independent workflow trust anchor and exact-authority regression to protected META `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, preserving Platform product/security/runtime behavior unchanged.

## Acceptance criteria

- [ ] Binding and independent workflow trust anchor equal exact protected META `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- [ ] Policy-consistency regression expects the same exact authority.
- [ ] Exact-head required Platform CI and independent review are clean.
- [ ] Integration uses protected Merge Queue, real merge-group `platform-gate`, and protected-main readback.

## Ownership

```yaml
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/active/OTERYN-20260924-meta-routing-authority-repin.md
modules:
  - agent-governance
dependencies:
  - Oteryn/Oteryn#224
blockers:
  - none
cross_repository_tasks:
  - META-INTEGRATION-ROUTING-REPIN
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-24T14:10:00Z
head: UNKNOWN
branch: governance/meta-routing-repin-1401
pr: 1412
status: validating
context_routes:
  - governance
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/active/OTERYN-20260924-meta-routing-authority-repin.md
proven:
  - protected META PR #224 integrated at 1bfb5ff98c8aa156e73669a14e083a1d464c29fb
  - protected META merge-group META CI succeeded on 1bfb5ff98c8aa156e73669a14e083a1d464c29fb
  - prior Platform publication-integrity adoption is already protected on main
derived:
  - current repin requires no Platform product/runtime/auth/payment/database/deployment change
unknown:
  - final exact candidate head
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/active/OTERYN-20260924-meta-routing-authority-repin.md
validation:
  - command: hosted Agent Governance / platform-gate
    result: NOT_RUN
    evidence: PR exact-head qualification pending
blockers:
  - none
next_action: freeze the successor exact head and consume fresh exact-head CI/review evidence
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository governance adoption PR
source_branch_evidence: pending protected integration
```

## Notes

This is an immutable authority/trust-anchor repin only. Do not copy META routing machinery into Platform, weaken protection, bypass Merge Queue, or use direct merge/generic auto-merge as a substitute.
