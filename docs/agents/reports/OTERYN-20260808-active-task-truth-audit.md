# Active task truth audit — 2026-08-08

## Result

`AUDIT_COMPLETE_WITH_FINDINGS`

Audited protected `main@0582b0e853d1b5e983f664452268e7777c886904` and the four active task records present at invocation start. Two new material task-evidence contradictions were proven and routed to Issues #876 and #877. Two other contradictions already had concrete delivery owners, so no duplicate Issues were created.

Before terminal validation, protected `main` advanced through native-protocol repair PR #875 and lifecycle closeout PR #879 to `9b84279dbd8a35a6f75ccd524daaf4a29e89b27a`. The final audit delivery is reconstructed directly on that current main. Those concurrent changes resolve the already-owned native protocol authority conflict and do not change the evidence supporting #876 or #877.

No runtime, workflow, deployment, credential, Cloudflare, Synology, production or external-repository mutation was performed.

## Audit matrix

| Active task | Evidence state | Audit disposition | Durable owner |
|---|---|---|---|
| `OTERYN-20260801-public-domain-repair` | `CONFLICT`, already owned | The audited-main checkpoint asks for obsolete Cloudflare-token/edge repair work, while PR #541 already contains the bounded reconciliation using later edge/HSTS evidence and leaves staging password-recovery delivery as the remaining criterion. No duplicate finding created. | PR #541 |
| `OTERYN-20260805-cloudflare-zone-edge-verification` | `CONFLICT` | The task preserves broad `UNKNOWN` HSTS and WAF/Bot state from an older 403 audit even though PR #516 records later trusted-main HSTS and WAF/Bot proof. Residual certificate-product/Access/Page Rule facts may remain unknown, but the task must be narrowed. | `OPA-GOV-0027`, Issue #877 |
| `OTERYN-20260805-native-auth-production-verification` | task checkpoint `PROVEN`; lower-level authority conflict repaired during audit | The task itself correctly routes target native authority to Oteryn-v2. Issue #874 already owned the lower-level Platform contract/operations conflict, PR #875 repaired it, and PR #879 archived/released the temporary repair task. No duplicate finding created. | closed Issue #874 / merged PR #875 / closeout PR #879 |
| `OTERYN-20260805-synology-staging-activation` | `CONFLICT` | The task says runner registration, Environment configuration, compatible Canary image and first deployment are unproven. PR #137 records a successful first deployment on the dedicated runner with an immutable Canary digest and independent verification; PR #141 and later staging deliveries provide further evidence. Historical proof does not establish today's secret values or runner liveness, but it disproves the stated never-activated premise. | `OPA-GOV-0026`, Issue #876 |

## Finding OPA-GOV-0026 — Synology activation task contradicts proven deployment history

**Severity:** HIGH  
**Confidence:** HIGH  
**Evidence:** PROVEN  
**Issue:** #876

The current task was created as a blocked activation-only continuation after Issue #566/PR #612. Its acceptance and checkpoint preserve runner registration, `synology-staging` Environment configuration, compatible Canary image and first controlled deployment as unproven gates.

Earlier terminal evidence already proves those activation events occurred:

- PR #137 records `Deploy Synology Staging` run `30075926039` successful on runner `oteryn-synology-staging` with label `oteryn-staging`;
- PR #137 records Platform/Gateway exact image identity, immutable Canary digest `sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f`, healthy probes and an independent runtime audit;
- PR #141 records a later guarded deployment and independent live verification on the same staging boundary;
- later exact-SHA staging work, including PR #267, proves the deployment path continued to be exercised.

The safe correction is lifecycle/evidence-only: archive the task if no distinct current objective remains, or narrow it to explicit *current-state revalidation*. Historical deployment evidence must not be converted into assumptions about present secret values or runner availability.

## Finding OPA-GOV-0027 — Cloudflare verification task ignores later edge proof

**Severity:** HIGH  
**Confidence:** HIGH  
**Evidence:** PROVEN  
**Issue:** #877

The verification task carries forward an older audit in which nine reads returned HTTP 403 and therefore classifies certificate, TLS, redirect, HSTS, WAF/Bot, Access and Page Rule state together as `UNKNOWN`.

That conclusion is stale for at least HSTS and WAF/Bot:

- PR #516 records guarded HSTS apply run `30855934824` reaching staged `max_age=2592000` with full public E2E PASS and positive HSTS;
- PR #516 records independent trusted-main audit run `30857136575` reproducing the target with `mutation=none`;
- PR #516 records the canonical WAF skip as first, Bot Fight Mode false and the unrelated broad country restriction preserved;
- PR #541 independently consumes this later evidence and treats the public edge/HSTS repair as complete.

The older 403 result still proves the limitations of that specific token. It does not make later successfully observed controls unknown. Any new protected token request must therefore be scoped only to residual facts that remain genuinely unproven.

## Existing finding ownership reused

### Public-domain repair

The audited `main@0582b0e853d1b5e983f664452268e7777c886904` checkpoint is stale, but PR #541 already contains the precise bounded reconciliation. Creating another Issue or PR would duplicate ownership. Audit disposition: `CONFLICT_ALREADY_OWNED`.

### Native-auth production verification

The active task itself is aligned with the Oteryn-v2 target. The conflict was below it: the Platform native gameplay contract still self-identified as `NORMATIVE` and the producer operations guide still routed rollout toward Otheryn on the initial audited main. Issue #874 already described that defect. PR #875 repaired it during this audit, and PR #879 archived/released the temporary repair task. Audit disposition: `REPAIRED_DURING_AUDIT`.

## Continuous-audit programme refresh

The previous programme checkpoint still described `OPA-GOV-0025` / Issue #848 as a live remediation handoff. Live state proves Issue #848 is closed and PR #854 merged the CI concurrency repair. The programme record is corrected in this audit package and the finding ledger is extended with `OPA-GOV-0026 -> #876` and `OPA-GOV-0027 -> #877`.

## E2E and safety

Runtime/browser E2E: `NOT_APPLICABLE` because this audit package changes only task/report/programme documentation and creates finding Issues. No executable behavior changes.

Production/staging mutation: `NOT_RUN`.  
Credential/secret mutation: `NOT_RUN`.  
External-repository write: `NOT_RUN`.
