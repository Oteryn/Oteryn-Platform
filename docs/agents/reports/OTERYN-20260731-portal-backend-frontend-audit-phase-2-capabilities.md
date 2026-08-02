# Portal backend/frontend audit — Phase 2 capability reconciliation

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Audit target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Result

The canonical benchmark reconciliation contains **43 capabilities**:

- `23` legacy `implemented`;
- `3` partial;
- `14` missing;
- `3` not applicable.

Static inspection found **zero** user-facing records where backend is `implemented` while frontend or integration is not implemented, and zero inverse frontend-only promotions. One implemented non-UI capability, support notification delivery, has an explicit bounded standalone-UI exception and is consumed through the integrated support lifecycle.

The strict backend/frontend validator was executed successfully:

- exact source: `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- Portal Acceptance Contract run: `30633216358`;
- job: `91164376176`;
- artifact: `8794204786`;
- result: `PASS`.

This corrects the earlier `UNKNOWN_NOT_EXECUTED` statement. Equivalence to the frozen audit target remains `DERIVED`, because the validator did not execute on exact frozen SHA `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`.

## Delivery-completeness policy-v2 boundary

The Phase 2 word `implemented` means only that backend, reachable frontend and real-route integration are recorded consistently in the legacy ledger. It does **not** mean complete under the current 13-gate delivery contract.

The policy-v2 overlay in Phase 6 records:

- `0` capabilities proven complete under all applicable gates;
- `23` repository-integrated capabilities with evidence/closeout open;
- `3` partial;
- `14` missing;
- `3` not applicable.

The 43 records are a benchmark subset, not an exhaustive module catalogue. CMS/content, Editorial Media, administrator/RBAC/audit, Platform API, legal/privacy/commerce, operations/observability, public edge and quality/E2E require explicit module-level records in the Phase 6 crosswalk.

Authoritative policy-v2 evidence:

- `docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/phase-6-delivery-completeness-crosswalk.json`;
- `docs/agents/reports/OTERYN-20260731-portal-backend-frontend-audit-phase-6-delivery-completeness.md`.

## Domain totals

| Domain | Total | Legacy implemented | Partial | Missing | Not applicable |
|---|---:|---:|---:|---:|---:|
| Account | 9 | 7 | 0 | 1 | 1 |
| Character | 9 | 5 | 0 | 4 | 0 |
| Commerce | 6 | 0 | 2 | 4 | 0 |
| Support | 4 | 4 | 0 | 0 | 0 |
| Public | 8 | 6 | 0 | 0 | 2 |
| Knowledge | 7 | 1 | 1 | 5 | 0 |

## Legacy integrated boundary

Legacy integrated records map to covered surface IDs with declared Playwright evidence markers. This proves the narrow backend/frontend/integration contract. It does not by itself prove every persistence, authorization, state, locale, accessibility, test, E2E, independent-audit, exact-head or terminal-PR/task gate.

The integrated boundary includes:

- account recovery, MFA, email change, sessions, privacy, recovery key and termination;
- owner character comments, privacy and main-character selection;
- public character, guild, highscore, deaths/kill and online/server reads;
- support tickets, reports, enforcement and background notification integration;
- Character Bazaar for the auction/wallet scope;
- first Game Catalog items/creatures/loot scope.

## Truthfully partial or missing boundary

No partial or missing capability is promoted to legacy implemented.

Character deletion/restore, rename, world transfer and achievement display remain missing. Existing ownership is tracked by Issues `#317`, `#319`, `#320` and `#323`.

Marketplace wallet and auction history are partial evidence for coin balance/history only. They do not establish customer payments, checkout, provider webhooks, refunds, chargebacks, premium/VIP, product delivery or redemption. Those remain under Issues `#321` and `#322`.

Broader knowledge capabilities remain missing or partial: spells, quests, NPCs, achievements, maps, calculators, optional discovery systems and world-transfer documentation.

## Environment boundary

Open-PR-only work is not promoted to repository-main delivery:

- PR `#338` adds inactive NPC/shop ingestion and administrator diagnostics but no public NPC/shop projection;
- the earlier rename discovery PR `#328` did not deliver a rename lifecycle and has since reached an intentional terminal state.

The frozen audit target, later repository main, staging and production remain separate evidence environments.

## Findings

### OTERYN-AUDIT-P2-001 — No backend-only/frontend-only legacy promotion

- fact state: `PROVEN`;
- severity: `INFO`;
- confidence: `HIGH`;
- result: the 43-record legacy ledger does not promote a one-sided user-facing implementation.

### OTERYN-AUDIT-P2-002 — Character lifecycle gaps are truthfully missing

- fact state: `PROVEN`;
- severity: `INFO`;
- result: product completeness is not achieved, but absent character lifecycles are not presented as delivered.

### OTERYN-AUDIT-P2-003 — Bazaar is not customer commerce

- fact state: `PROVEN`;
- severity: `INFO`;
- result: wallet/auction functionality remains separated from payment-provider and product-commerce claims.

### OTERYN-AUDIT-P2-004 — NPC/shop work is not repository-main public delivery

- fact state: `PROVEN`;
- severity: `INFO`;
- result: open or blocked producer work cannot be promoted to delivered public knowledge capability.

### OTERYN-AUDIT-P2-005 — Strict validator execution

- fact state: `PROVEN`;
- severity: `INFO`;
- exact source: `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- result: `PASS`;
- frozen-target relationship: `DERIVED_NOT_EXACT_HEAD`.

### OTERYN-AUDIT-P6-001 — Benchmark subset is not exhaustive module completion

- fact state: `PROVEN`;
- severity: `MEDIUM`;
- result: the legacy 43-record validator cannot fail closed across all 18 programme modules and all 13 delivery/closeout gates;
- owner: Issue `#326`, coordinated with programme `#451`;
- implementation: not authorized in this audit.

## Phase boundary

Phase 2 legacy reconciliation and its strict validator are complete for their bounded contract. Phase 6 supersedes any interpretation that legacy `implemented` means full delivery completion.
