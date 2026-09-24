---
task_id: OTERYN-20260924-bounded-connector-publication
governing_issue: 1410
required_reads:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
search_first:
  - publication integrity
optional_reads: []
---

# OTERYN-20260924-bounded-connector-publication

## Goal

Governing GitHub Issue: #1410 — canonical lifecycle authority for this task.

Adopt protected META publication policy `21bc49bccef4874b037aabcbde9732b904187c32` and reconcile Platform-local publication wording without widening runtime, production, credential or merge authority.

## Acceptance criteria

- [ ] Binding and independent workflow trust anchor equal exact protected META `21bc49bccef4874b037aabcbde9732b904187c32`.
- [ ] Bootstrap permits only the bound META API-native **new candidate** route when ordinary publication is unavailable, while ad-hoc raw Git Data/per-file reconstruction remains forbidden.
- [ ] Policy-consistency tests and authenticated provider validator pass against the exact bound META checkout.
- [ ] Exact-head required Platform CI and material review are clean.
- [ ] Integration uses governed/native Merge Queue, real merge-group `platform-gate`, and protected-main readback.

## Ownership

```yaml
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/active/OTERYN-20260924-bounded-connector-publication.md
modules:
  - agent-governance
dependencies:
  - Oteryn/Oteryn#223
  - Oteryn/Oteryn-Game#838
blockers:
  - none
cross_repository_tasks:
  - PUBLICATION-INTEGRITY-PROVIDER-ROLLOUT
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-24T13:45:00Z
head: 766acdbd4a6dd908d5bcbbd020b43fae5095957f
branch: governance/bounded-connector-publication-1410
pr: 1411
status: validating
context_routes:
  - governance
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/active/OTERYN-20260924-bounded-connector-publication.md
proven:
  - protected META publication policy revision is 21bc49bccef4874b037aabcbde9732b904187c32
  - Game-first provider adoption merged as Oteryn/Oteryn-Game#838 at 88efe885c1b9ff08dc3a33e061d9a63474d67824
  - protected Platform main at task start is 6a778ac19cea7fe83fafcb2b298d18b67a18fc71
derived:
  - Platform binding and independent workflow trust anchor must move together
unknown:
  - exact final candidate head and qualification result
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
  - docs/agents/tasks/active/OTERYN-20260924-bounded-connector-publication.md
validation:
  - command: exact-head Platform CI
    result: NOT_RUN
    evidence: fresh exact-head qualification pending after binding this packet to PR #1411
blockers:
  - none
next_action: rerun exact-head Platform qualification, review, and protected Merge Queue
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository governance adoption PR
source_branch_evidence: pending protected integration
```

## Notes

No product/runtime/auth/payment/database/deployment/production/secret/ruleset/protection mutation. The provider consumes META authority; it does not duplicate or widen it.
