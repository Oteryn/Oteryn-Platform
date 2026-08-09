---
task_id: OTERYN-20260809-federated-search-publication-revocation-fence
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
search_first:
  - Issue #938
  - PR #939 and PR #940
  - open repair/issue-938 branch or PR
  - active tasks and overlapping architecture ownership
optional_reads:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
---

# OTERYN-20260809-federated-search-publication-revocation-fence

## Goal

Repair OPA-SEC-0005 / Issue #938 by making newer restrictive source publication/visibility decisions deterministically fence older federated-search provider, derived-index, cache, web and future PlatformAPI representations without implementing search runtime.

## Acceptance criteria

- [x] Deterministic branch `repair/issue-938` claimed from trusted current `main`.
- [x] Repair remains architecture/documentation-only and preserves source-module publication authority.
- [x] Ordinary source/index/cache freshness is separated from publication/visibility-decision freshness.
- [x] Newer restrictive decisions override older allow/public representations even inside ordinary stale-index/cache tolerance.
- [x] Delayed/failed/ambiguous propagation fails closed for affected representations.
- [x] Publication-authority outage cannot silently reuse an unproven stale allow.
- [x] Rebuild/rollback cannot cross a newer restrictive-decision watermark.
- [x] Required negative-path validation scenarios are explicit.
- [ ] Open one Issue-owned repair PR and complete HEIGHTENED exact-head validation/review hygiene.
- [ ] Merge, verify resulting `main`, close Issue #938, archive this task and release the claim.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260809-federated-search-publication-revocation-fence.md
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
modules:
  - PublicPortal
  - federated-search architecture
  - security
  - architecture-governance
dependencies:
  - Issue #938
  - accepted ADR 0033
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-09T09:30:01Z
head: be0f65966dced9654f041987382f973588e6e5f6
branch: repair/issue-938
pr: none
status: implementing
context_routes:
  - agent-governance
  - architecture
  - security
  - public-web-cms
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260809-federated-search-publication-revocation-fence.md
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
proven:
  - Issue #938 is HIGH/P1, implementation-authorized, parallel-safe and owns the two federated-search architecture paths.
  - Deterministic branch repair/issue-938 did not exist before this session and was created successfully from main@e0c70b89963f55da3a95b6534728098596cc5001.
  - Open PR #945 owns only entitlement-audit documentation and explicitly forbids the Issue #938 paths.
  - The accepted architecture previously allowed bounded stale index/cache behavior without a restrictive publication-decision ordering fence.
  - The branch now defines a source-owned ordered restrictive-decision fence, fail-closed propagation/outage semantics and rollback/rebuild watermark requirements in both canonical documents.
derived:
  - Physical tombstone/cache eviction can complete after the public cutoff provided every delivery path already rejects the older representation.
  - A source-owned bounded publication proof/lease can avoid synchronous authority lookup only while its continuing authority is valid and testable.
unknown:
  - exact-head repository validation and PR review result after PR creation
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - ordinary cache TTL is sufficient authorization after a newer revoke
  - physical cache eviction alone is a reliable revocation cutoff
  - index generation alone proves current publication authority without a restrictive-decision ordering contract
changed_paths:
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
validation:
  - command: full architecture diff review
    result: NOT_RUN
    evidence: perform after claim/task activation and before PR readiness
blockers:
  - none
next_action: Recompute HEIGHTENED validation scope, review the complete branch diff against Issue #938 acceptance, then open the single Issue-owned repair PR.
```

## Notes

This repair does not claim a current runtime disclosure. Federated-search runtime/index/cache delivery remains separate future work.