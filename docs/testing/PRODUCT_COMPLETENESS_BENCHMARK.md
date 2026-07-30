# Oteryn Product Completeness Benchmark

## Audit identity

- Parent issue: #268
- Current reconciled repository baseline: `f90bb8075b300569b7d493c84f0080e6b3295c35`
- Detailed exact-SHA audit: `docs/testing/PRODUCT_COMPLETENESS_AUDIT_2026-07-29.md`
- Machine-readable capability ledger: `docs/testing/product-completeness-benchmark.json`
- Delivered-surface contract: `scripts/acceptance/coverage/portal-coverage-manifest.json` plus sorted fragments under `scripts/acceptance/coverage/surfaces/`
- Character Bazaar delivery: PR #270, merge `0f19656e0875d0a10b22002ac0e096deb20e94d8`
- Account-security lifecycle: PR #283
- Support/moderation lifecycle: PR #293
- Community-data completeness: PR #298
- First Game Catalog scope: PR #272, evidence closure PR #303
- Character profile preferences: PR #308, merge `86847d0068e470274b6c3ee5523fe41cbb9663af`
- PR #308 exact tested feature head: `3797a094cfa522f5147d624786f49fee5027c77b`

The JSON ledger preserves the original Issue #268 audit identity and the exact 43 capability records. The current repository identity, expanded classifications, child issue split and later evidence are recorded in the dated audit above.

## Verdict

The Oteryn delivered-surface route contract is broad, machine enforced and supported by exact repository and isolated staging-like evidence. It is **not** the same as product completeness against Tibia/RubinOT-style account, character, commerce, support, community-data and knowledge ecosystems.

**Benchmark product completeness is not achieved.**

Required product gaps remain:

- character deletion, grace period and restore — #317;
- conflict-safe rename, cooldown and history — #319.

Planned blockers and deferred capabilities remain:

- controlled world/channel transfer — #320;
- authoritative achievement catalogue/selection — #301 and #323;
- provider-neutral payment security foundation — #321;
- product, premium/VIP, coin purchase, voucher, entitlement and history lifecycle — #322;
- structured spell/NPC/quest/achievement catalogues — #301;
- optional maps, hunt tools and server-specific discovery — #302;
- optional loyalty/badge/status presentation — #325;
- exhaustive every-screen visual/state evidence closure — #326.

## Canonical ledger summary

The machine ledger contains **43 capabilities**.

| Delivery status | Count |
|---|---:|
| Implemented | 23 |
| Partial | 3 |
| Missing | 14 |
| Untested | 0 |
| Not applicable | 3 |

| Relevance | Count |
|---|---:|
| Required | 22 |
| Planned | 13 |
| Optional / differentiator | 5 |
| Not applicable | 3 |

Projection into the audit-request vocabulary:

| Requested status | Count |
|---|---:|
| `IMPLEMENTED` | 23 |
| `PARTIAL` | 3 |
| `MISSING_REQUIRED` | 2 |
| `MISSING_OPTIONAL` | 4 |
| `PLANNED` | 8 |
| `NOT_APPLICABLE` | 3 |
| `UNTESTED` | 0 |
| runtime `BROKEN` | 0 |

The dated audit records additional fine-grained `UNTESTED` and documentation `BROKEN` findings outside this canonical 43-item count.

## What is implemented

### Account and security

- registration, login/logout and throttling;
- password reset/change with single-use expiry and session revocation;
- TOTP MFA, recovery codes and challenge lifecycle;
- confirmed primary-email change, old-address recovery and cooldown;
- active-session inventory and targeted/global revocation;
- private-by-default account association/status controls;
- high-assurance recovery key lifecycle;
- Platform account termination grace, cancellation, anonymization/retention and audit.

Email-code MFA and mutable self-service game-account linking are intentionally not applicable to the approved security/binding model.

### Characters and public community data

- ready-binding character creation with validation, quota, transactions, idempotency and race evidence;
- account character inventory;
- public server-backed character profile, guild/house/status/skills/deaths/kills and privacy-aware related characters;
- owner comment, per-character visibility and optional single-main-character preference;
- highscores, character/guild search and detail, online players, servers, latest deaths and kill statistics.

### Publishing, support and administration

- Home, News, Managed Pages, Downloads, Events, Announcements and Support/Legal;
- public/editorial Wiki, media integrity and revision workflow;
- owner tickets, bounded reports, enforcement/appeals, notifications and retention;
- admin dashboard, exact MFA/RBAC, role assignment, CMS and audit;
- empty, not-found, authorization-denied and major dependency unavailable/restored states.

### Character Bazaar and first Game Catalog scope

- public/authenticated/admin Character Bazaar with filters, listing, bid, buy-now, wallet reservation, escrow, settlement, recovery, history and real-MariaDB concurrency;
- versioned server-backed item/weapon/creature/loot catalogue with provenance, immutable import, explicit activation and rollback.

## What is partial or absent

### Required

- deletion/grace/finalization/restore — #317;
- rename/reservation/cooldown/history — #319.

### Commerce

The Platform wallet exists for Character Bazaar. It is not a payment system. There is no selected payment provider, customer checkout, signed webhook lifecycle, refund/chargeback reconciliation, purchasable premium/coins, voucher/redeem code, entitlement delivery or complete commerce history. #321 and #322 are mandatory before commercial activation.

No fictional provider may be used. A deterministic test adapter is allowed only when it is impossible to enable as a production operator.

### Knowledge and differentiators

- structured spells, NPCs, quests, achievements and reward links — #301;
- selected public achievements — #323;
- maps/interactive maps, hunting tools/calculators, Equipment Presets, Huntfinder, Linked Tasks and Battle Pass/server-specific discovery — #302;
- complete authoritative server-system and annual-event catalogue — partial, #302;
- world-transfer documentation — blocked by #320.

External Wiki content is reference-only. It is never Oteryn server availability proof and may not be copied into the authoritative catalogue.

## Focused issue ownership

### Parent trackers

- #277 — character management/public-profile completion;
- #278 — commerce and entitlements;
- #301 — structured authoritative catalogue expansion;
- #302 — optional knowledge/discovery decisions;
- #91 — exact deployed production go-live verification.

### Bounded child trackers created by the 2026-07-29 reconciliation

- #317 — deletion/restore;
- #319 — rename;
- #320 — world/channel transfer;
- #321 — payment security/provider foundation;
- #322 — products/entitlements/vouchers/histories;
- #323 — achievement selection/profile display;
- #325 — loyalty/badge/status decision;
- #326 — exhaustive visual/state matrix.

Every child issue defines scope, data model, API/UI, security, migration/rollout, tests, acceptance criteria and dependencies. None authorizes Canary, payment-provider or production mutation.

## Exact evidence boundary

PR #308 exact final feature head `3797a094cfa522f5147d624786f49fee5027c77b` passed all 11 required workflows:

- CI `30490007511`;
- Agent Governance `30490007484`;
- Portal Acceptance Contract `30490007458`;
- Community Data Acceptance `30490007443`;
- Phase 7 Production-Like Validation `30490007483`;
- Platform DB Outage Validation `30490007507`;
- Edge Security Emulation `30490007432`;
- Game Auth Ticket Concurrency `30490007493`;
- Acceptance E2E and Visual UX `30490007509`;
- Synology Production Target Preflight `30490007537`;
- Build Synology Staging Images `30490007474`.

This is repository and isolated acceptance evidence. The latest separately documented deployed staging refresh predates later marketplace/account-security/catalog/support/community/profile deliveries. Current exact-main staging deployment is therefore not claimed.

## Claim boundary

- `CONTRACT_TESTED`: yes, for declared delivered surfaces;
- `PRODUCT_COMPLETE`: no;
- `STAGING_PROVEN`: only for specifically documented prior boundaries;
- `PRODUCTION_PROVEN`: no.

Issue #91 remains open. No repository, CI, browser harness, local-target preflight, edge emulation or staging-like evidence may be promoted to production proof.
