# Portal backend/frontend audit — Phase 2 capability reconciliation

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Audit target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Result

The canonical product/backend/frontend reconciliation contains **43 capabilities**:

- `23` implemented;
- `3` partial;
- `14` missing;
- `3` not applicable.

Static inspection found **zero** user-facing records where backend is `implemented` while frontend or integration is not implemented, and zero inverse frontend-only implemented promotions. One implemented non-UI capability, support notification delivery, has an explicit bounded standalone-UI exception and is consumed through the integrated support lifecycle.

The deterministic exact-target validator was not executed in this connector-only environment, so the machine execution result remains `UNKNOWN_NOT_EXECUTED`.

## Domain totals

| Domain | Total | Implemented | Partial | Missing | Not applicable |
|---|---:|---:|---:|---:|---:|
| Account | 9 | 7 | 0 | 1 | 1 |
| Character | 9 | 5 | 0 | 4 | 0 |
| Commerce | 6 | 0 | 2 | 4 | 0 |
| Support | 4 | 4 | 0 | 0 | 0 |
| Public | 8 | 6 | 0 | 0 | 2 |
| Knowledge | 7 | 1 | 1 | 5 | 0 |

## Implemented boundary

Implemented records map to covered surface IDs with declared Playwright evidence markers. The ledger does not by itself prove every state/browser permutation or deployment.

The implemented boundary includes:

- account recovery, MFA, email change, sessions, privacy, recovery key and termination;
- owner character comments, privacy and main-character selection;
- public character, guild, highscore, deaths/kill and online/server reads;
- support tickets, reports, enforcement and background notification integration;
- Character Bazaar for the auction/wallet scope;
- first Game Catalog items/creatures/loot scope.

## Truthfully partial or missing boundary

No partial/missing capability is promoted to implemented.

Character deletion/restore, rename, world transfer and achievement display remain missing. Under the task's intentional-gap rule these are recorded as `INFO`, not automatically as portal defects, because the backend/frontend ledger and frontend audit consistently identify them as absent and existing ownership is already tracked.

Marketplace wallet and auction history are partial evidence for coin balance/history only. They do not establish customer payments, checkout, provider webhooks, refunds, chargebacks, premium/VIP, product delivery or redemption.

Broader knowledge capabilities remain missing or partial: spells, quests, NPCs, achievements, maps, calculators, optional discovery systems and world-transfer documentation.

## Open-PR-only boundary

PR `#338` adds an inactive schema `1.3` NPC/shop consumer and bounded administrator candidate diagnostics. It adds no public NPC/shop projection and remains `OPEN_PR_ONLY`; therefore `knowledge.spells-quests-npcs-achievements` remains missing on `REPO_MAIN`.

PR `#328` is a character-rename architecture/discovery contract only. It does not deliver backend mutation or owner UI and remains `OPEN_PR_ONLY`; `character.rename` remains missing on `REPO_MAIN`.

## Findings

### OTERYN-AUDIT-P2-001 — No backend-only/frontend-only implemented promotion

- fact_state: `PROVEN`
- severity: `INFO`
- confidence: `HIGH`
- environment: `REPO_MAIN`
- impact: static ledgers do not misrepresent a one-sided user-facing implementation as complete.
- recommendation: preserve the fail-closed validator and obtain a fresh exact-target execution result.

### OTERYN-AUDIT-P2-002 — Character lifecycle gaps are truthfully missing

- fact_state: `PROVEN`
- severity: `INFO`
- confidence: `HIGH`
- environment: `REPO_MAIN`
- impact: benchmark product completeness is not achieved, but absent character lifecycles are not presented as delivered.
- recommendation: retain existing issue ownership; do not implement within this audit.

### OTERYN-AUDIT-P2-003 — Bazaar is not customer commerce

- fact_state: `PROVEN`
- severity: `INFO`
- confidence: `HIGH`
- environment: `REPO_MAIN`
- impact: Marketplace wallet/history must not be promoted to payment-provider or product-commerce completeness.
- recommendation: retain partial/missing commerce classifications pending separately authorized #321/#322 work.

### OTERYN-AUDIT-P2-004 — NPC/shop work exists only in open PR

- fact_state: `PROVEN`
- severity: `INFO`
- confidence: `HIGH`
- environment: `OPEN_PR_ONLY`
- impact: main, staging and production claims cannot include PR #338.
- recommendation: preserve environment separation.

### OTERYN-AUDIT-P2-005 — Exact validator not executed

- fact_state: `UNKNOWN`
- severity: `INFO`
- confidence: `HIGH`
- environment: `UNKNOWN`
- impact: static inspection cannot replace the deterministic cross-ledger validator.
- recommendation: run `npm --prefix scripts/acceptance run test:backend-frontend-completeness` in the frozen checkout.

## Phase boundary

Phase 2 static reconciliation is complete. Runtime controller/service/persistence traversal per capability and fresh validator execution remain for a checkout-capable session.
