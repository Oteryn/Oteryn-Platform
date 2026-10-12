---
task_id: OTERYN-20261012-decanary-docs-tier
governing_issue: 1474
required_reads: []
search_first:
  - docs/agents/CONTEXT_ROUTING.md
optional_reads: []
---

# OTERYN-20261012-decanary-docs-tier

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

DC-P1 DECANARY-DOCS-1 remainder (ruling Q5=A): add a capability-tier column to the routing table and label the remaining compatibility-path material in the data ownership document and the three compatibility contract headers as LEGACY / COMPATIBILITY (RETIRING). Labels only; no deletion and no contract semantic change. #1478 already marked the routes and contracts legacy and stated the target boundary, so this packet adds only the tier column and the retiring wording.

## Acceptance criteria

- [x] Routing table has a Tier column with an abstract tier legend and no concrete model names.
- [x] Compatibility sections of the data ownership document and the three compatibility contract headers carry the retiring label.
- [x] Legacy-token baseline not raised.

## Ownership

```yaml
owned_paths:
  - docs/agents/CONTEXT_ROUTING.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/contracts/IDENTITY_CANARY_ACCOUNT_BINDING_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20261012-decanary-docs-tier.md
  - docs/agents/tasks/archive/OTERYN-20261012-decanary-docs-tier.md
modules: []
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T09:00:00Z
head: UNKNOWN
branch: claude/decanary-docs-tier-20261012
pr: none
status: validating
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/CONTEXT_ROUTING.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/contracts/IDENTITY_CANARY_ACCOUNT_BINDING_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20261012-decanary-docs-tier.md
proven:
  - #1478 already labelled the routes and contracts LEGACY / COMPATIBILITY
derived:
  - tier assignment per route follows the plan rule (persistence, auth, wire format = T1; bounded code = T2)
unknown:
  - CI result until it runs on this head
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/CONTEXT_ROUTING.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/contracts/IDENTITY_CANARY_ACCOUNT_BINDING_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20261012-decanary-docs-tier.md
validation:
  - command: legacy_token_ratchet, adr_registry, checkpoint, documentation_ia, task_liveness
    result: PASS
    evidence: local run
blockers:
  - none
next_action: Archive this record once the PR is merged.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: pending
```

## Notes

Tier names are abstract; the concrete mapping stays in the META policy binding.
