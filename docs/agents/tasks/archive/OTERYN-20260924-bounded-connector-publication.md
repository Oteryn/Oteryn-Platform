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

Adopt current protected META authority `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, which contains bounded connector publication from Oteryn/Oteryn#223 and the subsequent integration-routing revision, and reconcile Platform-local publication wording without widening runtime, production, credential or merge authority.

## Acceptance criteria

- [x] Binding and independent workflow trust anchor equal exact current protected META `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- [x] Bootstrap permits only the bound META API-native **new candidate** route when ordinary publication is unavailable, while ad-hoc raw Git Data/per-file reconstruction remains forbidden.
- [x] Policy-consistency tests and authenticated provider validator pass against the exact bound META checkout.
- [x] Exact-head required Platform CI and material review are clean.
- [x] Integration uses governed/native Merge Queue, real merge-group `platform-gate`, and protected-main readback.

## Ownership

```yaml
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/archive/OTERYN-20260924-bounded-connector-publication.md
modules:
  - agent-governance
dependencies:
  - Oteryn/Oteryn#223
  - Oteryn/Oteryn#222
  - Oteryn/Oteryn-Game#838
blockers:
  - none
cross_repository_tasks:
  - PUBLICATION-INTEGRITY-PROVIDER-ROLLOUT
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-24T19:49:30Z
head: c2ed3400423794d95974a44d8ac296956193192a
branch: governance/bounded-connector-publication-1410
pr: 1411
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - governance
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/archive/OTERYN-20260924-bounded-connector-publication.md
proven:
  - protected META publication policy revision 21bc49bccef4874b037aabcbde9732b904187c32 is an ancestor of current protected META 1bfb5ff98c8aa156e73669a14e083a1d464c29fb
  - current protected META authority 1bfb5ff98c8aa156e73669a14e083a1d464c29fb includes both bounded connector publication and inherited integration routing
  - Game-first provider adoption merged as Oteryn/Oteryn-Game#838 at 88efe885c1b9ff08dc3a33e061d9a63474d67824
  - protected Platform main at task start was 6a778ac19cea7fe83fafcb2b298d18b67a18fc71
  - protected Platform main advanced to 2134a2f59d24a7e77fac2730fe97f280506e652f with the final META authority repin before integration
  - PR #1411 exact head c2ed3400423794d95974a44d8ac296956193192a passed required Platform CI and entered governed Merge Queue
  - PR #1411 merged through real merge-group as protected main 76fa0ddfefc448743d368f4ad1da43642db2b120
  - post-merge independent review identified one remaining indented-Setext continuation P2; this archive/repair successor carries the fail-closed fix and must itself qualify before issue closure
derived:
  - Platform binding and independent workflow trust anchor must remain on the newer final META authority while publication fallback semantics are reconciled
unknown: []
conflicts:
  - resolved by merge-up: protected main changed the same META binding/workflow/test paths; newer authority retained
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - .github/workflows/agent-governance.yml
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - tools/agents/test_policy_consistency.py
  - docs/agents/tasks/archive/OTERYN-20260924-bounded-connector-publication.md
validation:
  - command: exact-head Platform CI on c2ed3400423794d95974a44d8ac296956193192a
    result: PASS
    evidence: checkpoint-validation, platform-gate and CodeQL completed success before integration
  - command: protected Merge Queue integration
    result: PASS
    evidence: Merge Queue UUID 7bbbe7a8-4675-4e81-9e9f-7f539fbf24e9; merge-group run 36050603860 platform-gate success; PR #1411 merged as 76fa0ddfefc448743d368f4ad1da43642db2b120
blockers:
  - none
next_action: archive complete; close governing issue after protected-main readback of the archive/repair successor
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository governance adoption PR
source_branch_evidence: live GitHub readback returned 404 Branch not found for governance/bounded-connector-publication-1410 after PR #1411 merged as 76fa0ddfefc448743d368f4ad1da43642db2b120
```

## Notes

No product/runtime/auth/payment/database/deployment/production/secret/ruleset/protection mutation. The provider consumes META authority; it does not duplicate or widen it.
