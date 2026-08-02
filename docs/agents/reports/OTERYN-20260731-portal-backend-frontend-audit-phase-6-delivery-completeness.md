# Portal backend/frontend audit — Phase 6 delivery-completeness policy v2

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Policy source inspected on current `main`: `39bdf0c79ffb0f7fd8daafd5451b9ad4e520138c`  
Programme: Issue `#451`  
Parent acceptance issue: `#326`  
Related defect evidence: Issue `#365`

## Scope and authority

This phase is audit-only. It does not authorize product implementation, workflow changes, merge of temporary validation infrastructure, deployment, production mutation or work in another repository.

The purpose is to reinterpret the existing portal evidence under the current delivery-completeness contract without discarding valid legacy evidence.

## Reasoning

The earlier Phase 2 ledger answers a narrow question: whether 43 benchmark capabilities have backend, frontend and real-route integration without one-sided promotion.

The current policy requires a broader conclusion. A user-facing capability is complete only when all applicable persistence, backend, authorization, transport, real frontend, observable states, localization, responsive/accessibility, focused/integration tests, real zero-retry E2E, independent audit, exact-final-head CI and related-PR/task terminal-state gates are proven together.

The merged production-completion baseline also identifies 18 platform modules. Several delivered or cross-cutting modules are not represented as explicit records in the 43-capability ledger.

## Corrected interpretation

Legacy result:

- 43 benchmark capabilities;
- 23 `implemented`;
- 3 `partial`;
- 14 `missing`;
- 3 `not_applicable`;
- zero backend-only or frontend-only promotions to `implemented`.

Policy-v2 result:

- **0 capabilities proven complete under all applicable gates**;
- 23 repository-integrated capabilities with evidence/closeout still open;
- 3 partial;
- 14 missing;
- 3 not applicable with an existing rationale.

This does not retract the 23 repository integration findings. It prevents the legacy word `implemented` from being misread as full production/delivery completion.

## Why the complete count is zero

For legacy `implemented` records, the current evidence is not normalized per capability across all required layers:

- persistence applicability is not explicit per record;
- authorization and server-side validation are not explicit per record;
- state coverage is bounded rather than exhaustive;
- accessibility is representative rather than fail-closed per surface;
- focused and integration tests are referenced but not normalized into one per-capability contract;
- zero-retry E2E proves a broad critical profile, not every applicable state;
- the independent audit still has material medium findings;
- strict backend/frontend validation passed on source `fdb45a4325949d3ab1c4860e3a4527553f11c789`, not the exact frozen target;
- related task and PR terminal-state evidence is incomplete while `#326`, `#365`, PR `#381` and temporary PR `#476` remain open or active.

Unknown evidence is not promoted to proven evidence.

## Module crosswalk

| Module | Repository/programme baseline | Policy-v2 status | Principal open gates |
|---|---|---|---|
| Identity | implemented repository-proven | repository-integrated, evidence open | per-capability 13-layer proof; exhaustive states/accessibility; exact target |
| Accounts | implemented repository-proven | repository-integrated, evidence open | optional badge gap; exhaustive evidence; exact target |
| Characters | partial | partial | deletion/restore, rename, transfer, achievements, cross-repository contracts |
| Public game data | implemented repository-proven | repository-integrated, evidence open | content-scale fragment omission; accessibility; runtime freshness |
| CMS/content | implemented repository-proven | repository-integrated, evidence open | no explicit 43-ledger records; module-catalogue drift |
| Editorial media | implemented repository-proven | integrated with material findings open | fixture isolation and scoped deterministic media evidence |
| Wiki | implemented repository-proven | integrated with material findings open | Issue #365, invalid HTML pattern, fixture isolation |
| Support/moderation | implemented repository-proven | repository-integrated, evidence open | content-scale omission; accessibility; production notifications |
| Admin/RBAC/audit | implemented repository-proven | repository-integrated, evidence open | no explicit 43-ledger records; operator/production access proof |
| Wallet/Marketplace | implemented repository-proven | partial | real-money funding and full customer commerce remain absent |
| Game Catalog | partial | partial | NPC/shop, spells, quests, achievements, maps and producer rollout |
| Platform API | missing later | missing later | stable API auth, versioning and rate limits |
| Payments | missing required | missing required | provider-neutral payment foundation and activation gates |
| Products/entitlements | missing required | missing required | products, premium/VIP, vouchers, fulfilment and histories |
| Legal/privacy/commerce | missing required | missing required | Poland/EU commerce, retention, refund and tax boundaries |
| Operations/observability | partial | partial/blocked | exact release, mail/queue/session/cache, observability, restore/rollback |
| Public edge | blocked | blocked | DNS/TLS/redirect/HSTS/origin/private-ingress evidence |
| Quality/E2E | partial | partial | every-screen/state closure, Issue #365 and deployed private-production E2E |

Machine-readable details are in `phase-6-delivery-completeness-crosswalk.json`.

## New finding

### MEDIUM — OTERYN-AUDIT-P6-001

**The 43-capability backend/frontend ledger is a benchmark subset, not an exhaustive portal/module completion ledger.**

Proven omissions at explicit capability-record level include CMS/content, Editorial Media, administrator/RBAC/audit, Platform API, legal/privacy/commerce, operations/observability, public edge and quality/E2E. Some of these exist in route/surface or programme ledgers, but the current strict backend/frontend validator cannot fail closed on their full 13-layer completion.

Impact:

- a module may exist in the route/surface inventory without an explicit completion record;
- a legacy `implemented` capability can be misread as fully complete after policy v2;
- cross-cutting operational and closeout gates are not attached to each user-facing capability.

Disposition: open under Issue `#326`, coordinated with programme `#451`. This audit records the gap only; another agent owns implementation.

## Corrected stale evidence

### INFO — OTERYN-AUDIT-P6-002

Phase 2 retained `runtime_validator_status: UNKNOWN_NOT_EXECUTED` after the strict Portal Acceptance Contract passed:

- source: `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- run: `30633216358`;
- job: `91164376176`;
- artifact: `8794204786`;
- result: `PASS`.

This proves the strict validator on that exact source. Equivalence to the frozen target remains `DERIVED`, not exact-head proof.

The Phase 2 report and machine-readable artifact must be synchronized; the Phase 6 crosswalk is authoritative until that correction is complete.

## Finding totals

Open normalized findings after the policy-v2 extension:

- HIGH: `0`;
- MEDIUM: `7`;
- LOW: `1`.

The new medium finding is `OTERYN-AUDIT-P6-001`. `OTERYN-AUDIT-P6-002` is a corrected audit-artifact contradiction and is not counted as open.

## Boundary for the implementation agent

The implementation agent should extend the existing Issue `#326` evidence contract rather than create a competing product ledger. It must:

1. reconcile all 18 programme modules and every adopted capability;
2. attach the applicable 13 delivery layers;
3. classify non-UI exceptions explicitly;
4. fail on missing module/capability records;
5. link exact focused, integration, browser, independent-audit, exact-head CI and terminal-PR/task evidence;
6. preserve `partial`, `missing`, `blocked` and `not_applicable` truthfully.

No such implementation is performed in PR `#381`.

## Conclusion

The portal is broad and substantially repository-integrated. It is not proven complete under policy v2.

The authoritative wording is:

```yaml
legacy_backend_frontend_result:
  implemented: 23
  partial: 3
  missing: 14
  not_applicable: 3
policy_v2_result:
  complete: 0
  repository_integrated_evidence_open: 23
  partial: 3
  missing: 14
  not_applicable: 3
```

Issue #365 and the exhaustive Issue #326 evidence matrix remain material completion gates.
