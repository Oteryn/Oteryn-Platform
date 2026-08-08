# Character lifecycle authority audit — 2026-08-08

## Result

`AUDIT_COMPLETE_WITH_FINDINGS`

Audited the retained character-management backlog on protected `main@6fb22e7518651b2c340442a3857eef9b6aefa856` against accepted ADR 0030 and ADR 0031.

One coherent material contradiction was proven and routed to `OPA-GOV-0030` / Issue #890. No runtime, database, workflow, deployment, credential, production, Canary or Oteryn-v2 mutation was performed.

## Authority baseline

Accepted ADR 0030 establishes Oteryn-v2 Character Authority as the native owner for canonical `CharacterId`, current `AccountId <-> CharacterId` ownership and native create/rename/delete/restore/world-transfer/account-transfer mutation outcomes. Platform `Characters` orchestrates versioned game-owned commands and consumes bounded receipts/projections.

Accepted ADR 0031 preserves current Canary behavior as `Legacy Canary Compatibility` while prohibiting direct/shared game SQL, Canary numeric identifiers or operation-specific legacy writes from becoming the unqualified target-native steady-state design.

## Audited backlog

| Issue | Current instruction | Audit disposition |
|---:|---|---|
| #277 | Parent still frames remaining lifecycle work around operation-specific Canary mutation contracts. | Requires authority reconciliation. |
| #317 | Future deletion/restore uses Canary player/account references, Platform/Canary write contract, dedicated Canary SQL principal and Identity-to-Canary/session dependencies. | Material target-authority drift. |
| #319 | Future rename uses Platform/Canary mutation contract, Canary uniqueness/index assumptions and Canary-commit recovery. | Material target-authority drift. |
| #320 | Future world/channel transfer requires Platform/Canary source-of-truth contract and possible Canary producer change. | Material target-authority drift unless explicitly compatibility-only. |
| #324 | Open task explicitly defines a `Canary-safe character rename contract`. | Historical/current compatibility work must be reclassified before target-native dispatch. |
| #344 | Makes a new Canary-owned deletion lifecycle the prerequisite that unblocks #317. | Cannot remain the unqualified native prerequisite after ADR 0030/0031. |

## Finding OPA-GOV-0030

**Issue:** #890  
**Severity:** HIGH  
**Priority:** P1  
**Confidence:** HIGH  
**Evidence:** PROVEN

The affected issues were valid architecture/planning artifacts for the Canary-compatible generation in which they were created. After acceptance of ADR 0030 and ADR 0031, they remain useful only when the Canary path is explicitly classified as Legacy Canary Compatibility or migration evidence.

The current wording is unsafe because a future autonomous worker can select a required character-lifecycle gap and implement new Platform-to-Canary SQL coupling as if it were the canonical future architecture. For rename, deletion/restore and world transfer this can create duplicate authority, stale identifier dependencies and destructive cross-system behavior that conflicts with the accepted Oteryn-v2 target.

The safe remediation is not to erase Canary evidence. It is to reconcile the backlog so that:

- native target work routes to Oteryn-v2 Character Authority command/receipt boundaries;
- retained Canary implementations are explicitly compatibility-only and separately activated;
- direct/shared SQL is not presented as native steady state;
- historical Canary discovery remains available for migration and current-runtime compatibility decisions.

## Overlap and deduplication

- Issue #888 owns native pre-admission/session handoff semantics; it does not own character lifecycle mutation commands.
- Issue #886 owns stale OTClient authority in PR #391; it does not own rename/deletion/world-transfer backlog semantics.
- Issues #876, #877 and #885 plus PR #541 retain their independent infrastructure/edge/lifecycle ownership.
- No existing open finding owned this exact cross-issue character-lifecycle authority contradiction before #890.

A mistaken empty duplicate Issue #891 was created during audit bookkeeping and was immediately closed with `state_reason=duplicate`; canonical ownership remains Issue #890.

## Validation disposition

- Runtime/application build: `NOT_APPLICABLE` — no executable behavior changed.
- Browser/runtime E2E: `NOT_APPLICABLE`.
- External repositories: read-only evidence only; no mutation.
- Required final evidence: exact-head Agent Governance, repository-selected CI, complete three-path diff review, zero unresolved review threads, merge and lifecycle closeout.
