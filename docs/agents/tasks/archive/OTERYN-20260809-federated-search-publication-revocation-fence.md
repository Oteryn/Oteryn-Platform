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
  - PR #947
optional_reads:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
---

# OTERYN-20260809-federated-search-publication-revocation-fence

## Goal

Repair OPA-SEC-0005 / Issue #938 by making newer restrictive source publication/visibility decisions deterministically fence older federated-search provider, derived-index, cache, web and future PlatformAPI representations without implementing search runtime.

## Terminal outcome

`COMPLETE / MERGED / ARCHIVED`

Issue #938 is closed as completed. PR #947 squash-merged into protected `main` as `a82ec651f9155fc5acbfe78d6c3b792fa9b9c0b8`.

The accepted architecture now requires:

- publication/visibility-decision freshness to remain distinct from ordinary source/index/cache freshness;
- source-owned monotonic or equivalently strong restrictive-decision ordering evidence;
- the restrictive fence/watermark to advance before or atomically with the effective deny;
- a newer unpublish/revoke/delete/moderation/legal-removal/incompatibility decision to make every older affected provider/index/cache/web/future PlatformAPI representation unservable regardless ordinary TTL or tolerated index lag;
- out-of-order older allow/update state to be unable to regress a newer deny;
- delayed, failed or ambiguous tombstone/index/cache propagation to fail closed for affected representations;
- any non-synchronous/cached serving path to prove the current restrictive fence; a time-expiring allow lease alone is explicitly insufficient revocation safety;
- publication-authority outage with unprovable current restrictive-fence state to produce unavailable/fail-closed behavior rather than stale public-by-default behavior;
- rebuild/cutover/rollback to be unable to activate an index generation behind a newer restrictive-decision watermark.

No federated-search runtime, route, schema, cache/index implementation, deployment, production activation or external-repository mutation was authorized or delivered by this task.

## Validation and review evidence

- validation intensity: `HEIGHTENED` because Issue #938 was HIGH/P1 and governs a durable public/security revocation boundary plus rollback semantics;
- first full-diff self-review: `FAIL`, correctly found the stale pre-revocation time-lease ambiguity;
- repair: both canonical architecture documents were tightened so current restrictive-fence proof remains mandatory and a time-based allow lease cannot survive a newer deny as authorization;
- Codex review: found the same material P1 on an earlier head; the finding was repaired and its thread resolved with exact contract evidence;
- final exact-head repair self-review at `2fd17a6da9856ad823ec3192b0bc2c4178c2a2b1`: `PASS`, zero remaining material findings;
- protected-main synchronization preserved the repair and independently merged entitlement-audit closeout, producing exact PR head `aa35c0f611aca515bb53caa83496454e629f22b5`;
- final exact-head self-review at `aa35c0f611aca515bb53caa83496454e629f22b5`: `PASS`, zero remaining material findings;
- exact-head workflows on `aa35c0f611aca515bb53caa83496454e629f22b5`: CI `31306600758` PASS; Agent Governance `31306600768` PASS; Platform DB Outage `31306600774` PASS; Native protocol contract `31306600757` PASS; Native protocol audits `31306600766` PASS; Game Auth Ticket Concurrency `31306600767` PASS; Edge Security Emulation `31306600759` PASS; Phase 7 Production-Like Validation `31306600786` PASS;
- review threads at merge: zero unresolved material threads;
- GitHub mergeability immediately before merge: `clean` against current protected `main@c1b1d26b355db26a89d983cc4abc6477bf843a26`;
- resulting protected main: `a82ec651f9155fc5acbfe78d6c3b792fa9b9c0b8`;
- Issue #938: `closed/completed` at 2026-08-09T09:51:22Z;
- runtime/browser E2E: `NOT_APPLICABLE` because the repair is architecture/documentation-only.

## Lifecycle closeout

```yaml
checkpoint_version: 1
updated_at: 2026-08-09T09:52:00Z
status: complete
phase: archived
branch: closeout/issue-938
repair_pr: 947
repair_head: aa35c0f611aca515bb53caa83496454e629f22b5
merge_sha: a82ec651f9155fc5acbfe78d6c3b792fa9b9c0b8
issue: 938
issue_state: closed
validation: PASS
material_findings_open: 0
runtime_implementation_authorized: false
production_activation_authorized: false
claim_release: pending_closeout_merge
next_action: Merge the lifecycle-only closeout PR, then release the Issue #938 repair claim and leave the task archived.
```

## Former ownership

The task formerly owned only:

- `docs/agents/tasks/active/OTERYN-20260809-federated-search-publication-revocation-fence.md`;
- `docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md`;
- `docs/architecture/adr/0033-federated-content-search-and-discoverability.md`.

After this lifecycle closeout merges, no active task ownership remains for those paths under Issue #938. Future federated-search implementation requires a new bounded task and must consume the accepted restrictive-decision fence rather than reopening this repair implicitly.
