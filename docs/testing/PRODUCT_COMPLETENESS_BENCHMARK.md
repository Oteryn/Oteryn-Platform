# Oteryn Product Completeness Benchmark

## Audit identity

- Parent issue: #268
- Repository baseline: `0a9a00014f55d7b2146d4ab151cd2a1b7c5bcd3d`
- Machine-readable ledger: `docs/testing/product-completeness-benchmark.json`
- Delivered-surface contract: `scripts/acceptance/coverage/portal-coverage-manifest.json` plus sorted fragments under `scripts/acceptance/coverage/surfaces/`
- Character Bazaar delivery: PR #270, merge `0f19656e0875d0a10b22002ac0e096deb20e94d8`
- Account-security lifecycle delivery: PR #283

## Verdict

The current Oteryn delivered-surface contract is broad and its declared routes have exact repository and isolated staging-like acceptance evidence. That is not the same as product completeness against Tibia/RubinOT-style account, character, commerce, support, community-data and knowledge ecosystems.

The benchmark ledger contains **43 capabilities**:

| Delivery status | Count |
|---|---:|
| Implemented | 9 |
| Partial | 8 |
| Missing | 25 |
| Untested | 0 |
| Not applicable | 1 |

Relevance classification:

| Relevance | Count |
|---|---:|
| Required | 22 |
| Planned | 13 |
| Optional / differentiator | 7 |
| Not applicable | 1 |

**Oteryn must not claim benchmark product completeness while required partial or missing capabilities remain open.** The principal required-gap trackers are #277, #279 and #280. Commerce is intentionally planned rather than part of the current non-commercial launch boundary, but #278 is mandatory before any commercial activation. Structured server-backed Wiki expansion is tracked by #281.

Issue #276 now has a delivered account-security lifecycle for the approved Platform boundary. It does not authorize Canary account unlink/rebind, native game-account deletion, character deletion or production deployment.

This report does not establish deployment to production or `PRODUCTION_PROVEN` status. Production verification remains independently owned by #91.

## Method

The audit does not infer completeness from a green route manifest. It combines four evidence classes:

1. **Actual application structure** — named routes, controllers, repositories, models, views and migrations.
2. **Delivered-surface contract** — role, state, viewport, browser and evidence declarations from the portal coverage manifest and module fragments.
3. **Executable evidence** — feature, integration, concurrency and browser acceptance files referenced by stable marker.
4. **External product benchmark** — official Tibia account/community/store/support surfaces and RubinOT/Tibia Wiki/Tibiopedia feature categories supplied in #268.

Delivery status meanings:

- `implemented` — the benchmark capability is materially complete inside the stated Oteryn boundary and has repository or browser evidence;
- `partial` — a real subset exists, but a benchmark-significant lifecycle or state is absent;
- `missing` — no delivered Oteryn lifecycle satisfies the capability;
- `untested` — code appears present but sufficient evidence is not available;
- `not_applicable` — explicitly rejected by Oteryn product policy with a durable rationale.

Relevance meanings:

- `required` — necessary for the intended complete Oteryn portal/account experience;
- `planned` — intentionally deferred product capability with a tracked delivery path;
- `optional_differentiator` — useful competitor-style enhancement that is not required for initial completeness;
- `not_applicable` — intentionally outside Oteryn product scope.

## Current delivered route and state inventory

The inventory is based on the exact named-route contract rather than screenshots or menu labels. Every delivered surface includes declared authorization roles and relevant empty, validation, denial, not-found, dependency or recovery states.

| Surface | Current delivery | Principal states and boundaries |
|---|---|---|
| Identity registration, login and logout | Covered | initial, validation, duplicate email, invalid credentials, redirect, logout, protected-route redirect |
| Password recovery and change | Covered | SMTP delivery, reset success, invalid/expired token, replay denial, wrong current password, global session revocation |
| TOTP MFA lifecycle | Covered | enrollment, QR provisioning, confirmation, challenge, invalid code, replay denial, recovery-code single use, disable, session invalidation |
| Account overview and Canary provisioning | Covered | ready, pending, recoverable, conflict, missing, retry, internal identifiers hidden |
| Account security and lifecycle | Covered | EN/PL, confirmed email change and old-address recovery, cooldown, active-session inventory, targeted/current/all-other revocation, privacy, recovery-key issue/revoke/use/replay denial, termination grace/cancel/finalize, stale-session redirect |
| Character creation and visibility | Covered | validation, reserved/duplicate name, quota, ownership injection denial, idempotent outcome, public visibility |
| Public home and SEO | Covered | available, empty, stale, dependency unavailable, EN/PL, published-only sitemap/robots |
| Public news and managed pages | Covered | published, empty, not found, unpublished hidden, long content, EN/PL |
| Public game data | Covered for delivered read model | highscores, character search/detail, guild detail, online, servers, pagination, empty, not found, dependency unavailable/restored |
| Core admin, RBAC, CMS and audit | Covered | guest/no-MFA/no-permission denial, exact role assignment/removal, final-admin protection, publication and audit |
| Localization administration | Covered | EN/PL, missing/incomplete/draft/published/stale translation and route-preserving switch |
| Downloads | Covered | empty/current/filter, validation, draft/publish, URL denial, dependency recovery, audit, EN/PL |
| Events | Covered | empty/upcoming/active/archived/cancelled/not found, draft/publish, stale conflict, permission denial, audit, EN/PL |
| Announcements | Covered | no active/active/future/expired/draft, escaped text, localization staleness, conflict, permission and audit |
| Support and legal content | Covered as CMS content only | published/unpublished/missing, legal version, approved links, validation, permission, EN/PL; no ticket or moderation lifecycle |
| Public Wiki | Covered | home/category/article/search, empty/invalid search, not found, unavailable/recovery, EN/PL |
| Wiki administration | Covered | draft/review/publish/unpublish/archive, revision restore, stale conflict, signed preview, permission and audit |
| Editorial media administration | Covered | empty/upload/validation/integrity rejection/delete/reference protection/permission |
| Media and Wiki preview endpoints | Supporting endpoints | authorized, unreferenced, draft hidden, integrity failure, invalid signature, not found |
| Character Bazaar public catalogue/detail | Covered | active/filter/empty/detail/immutable snapshot/history/not found/EN/PL/private saga hidden |
| Character Bazaar account lifecycle | Covered | wallet/reservation, watch, validation, escrow, bid/outbid, buy-now, cancel/expiry, history, recovery, idempotency |
| Character Bazaar administration | Covered | MFA/permission denial, wallet lookup/adjustment, ledger/audit, empty recovery queue, bounded retry, ownership conflict |

The route contract proves that delivered surfaces are classified. It does not prove that absent character, support, commerce or community capabilities are acceptable omissions.

## Benchmark results by domain

### Account and security

Implemented:

- secure token-based password recovery/change with token replay denial and global session revocation;
- TOTP enrollment/challenge/disable and recovery codes; email-code MFA is intentionally not adopted because email is the recovery channel;
- immutable `1 Platform Identity <-> 1 Canary account` greenfield binding with visible provisioning state;
- confirmed primary-email change, new-address confirmation, old-address cancellation/recovery, cooldown and global authorization revocation;
- active registered-session inventory with owner-scoped targeted, current and all-other revocation;
- account-level public-association and status privacy controls that default to private;
- verifier-only high-assurance recovery key generation, rotation, revocation, single use and replay denial;
- bounded Platform account termination with grace period, cancellation, audited finalization and preservation of Canary-owned data;
- English and Polish account-security UI, validation, token errors and notification links.

Policy decision classified `not_applicable`:

- self-service Canary account unlink/rebind/transfer. The ready binding remains immutable because no safe user-facing mutation contract exists; exceptional mutation requires a separately reviewed operation contract.

Optional remaining gap:

- account badges/loyalty/status presentation.

Issue #276 is delivered for the approved Platform-owned scope. Native Canary account lifecycle or a future exceptional binding operation remains a separate contract, not a hidden gap in this delivered surface.

### Character management and public profile

Implemented subset:

- character creation with validation, quotas, ownership controls and public visibility;
- basic active-character public detail containing name, level, vocation and guild name;
- Character Bazaar ownership transfer through escrow, which is not a world-transfer or general owner-management service.

Required gaps:

- editable public information/comment and moderation-safe rendering;
- character privacy controls;
- complete public profile including applicable deaths, house, achievements and account linkage;
- deletion grace period and restore;
- rename with history/cooldown and cross-surface consistency;
- complete guild/house/account visibility rules.

Planned/optional gaps:

- world or channel transfer;
- achievement selection;
- main-character selection.

Trackers: **#277**, with public read-model overlap in **#280**.

### Commerce and entitlement

The Character Bazaar wallet is a secure Platform-owned reservation and settlement ledger. It is not a customer commerce system.

Missing planned lifecycle:

- premium/VIP products and expiry;
- customer coin purchases and provider delivery;
- game-code/voucher redemption;
- purchased-products-ready-to-use state;
- payment/store/coin/service/voucher histories;
- provider checkout, signed webhooks, replay/idempotency, refund and chargeback reconciliation.

Tracker: **#278**. This issue is a mandatory gate before enabling commercial payments, even though commerce is not required for the present non-commercial portal boundary.

### Support, moderation and enforcement

Static localized support/legal/report-a-bug pages are complete as CMS content. They do not provide operational support or moderation.

Missing required lifecycle:

- authenticated tickets, replies, statuses and closure;
- report submission, user history and pending limits;
- moderator queues and outcomes;
- account-visible warning/punishment/rule-violation history;
- ticket/report/enforcement notifications and privacy-safe audit.

Tracker: **#279**.

### Public and community data

Implemented:

- online players and server/channel status with dependency recovery states;
- Character Bazaar public/account/admin lifecycle;
- basic character, guild and level-highscore reads.

Required partial or missing capabilities:

- complete character search/detail read model;
- guild directory/search and authorized guild administration;
- highscores categories and filters beyond level;
- latest deaths and relevant kill statistics;
- transfer history when transfer is introduced.

Optional policy-dependent capabilities:

- polls;
- public ban/punishment information.

Tracker: **#280**, with character mutation/profile overlap in **#277** and enforcement ownership in **#279**.

### Knowledge and tooling ecosystem

The current Wiki has a strong public/editorial workflow, localization, revisions, signed previews and media controls. Its content model is primarily editorial, not an authoritative server-backed gameplay catalogue.

Planned foundation:

- Oteryn-available creatures/monsters;
- items, weapons and equipment with exact server parameters;
- loot relations between creatures and items;
- structured spells, quests, NPCs and achievements where contracts exist;
- versioned source revision and provenance so removed server content cannot remain silently current;
- server-specific systems/items/events and world-transfer documentation when applicable.

Optional differentiators after the foundation:

- maps and interactive maps;
- hunting-place discovery and calculators;
- equipment presets;
- Huntfinder-like matching and linked tasks;
- battle-pass and other server-specific engagement systems.

Tracker: **#281**.

## Gap backlog and priority

| Priority boundary | Issue | Scope |
|---|---|---|
| Delivered account/security lifecycle | #276 | Confirmed email, sessions, privacy, recovery key, termination and explicit MFA/binding policy |
| Required character/profile completeness | #277 | Public profile editing/privacy, delete/restore, rename, linkage and transfer policy |
| Required support/moderation completeness | #279 | Tickets, reports, enforcement history and notifications |
| Required community-data completeness | #280 | Rich profiles, guild workflows, highscore filters, deaths and statistics |
| Mandatory before commercial activation | #278 | Premium, coins, products, provider/webhook/refund/chargeback lifecycle |
| Planned knowledge-platform expansion | #281 | Server-backed creature/item/loot and gameplay catalogues, then optional tooling |
| Separate presentation enhancement | #244 | Audited administrator homepage-template selector |
| Separate production gate | #91 | Exact deployed production verification; not satisfied by this audit |

## External benchmark references

Primary references used to define capabilities rather than to copy code, markup, prose or assets:

- `https://www.tibia.com/account/`
- `https://www.tibia.com/community/`
- `https://www.tibia.com/support/?subtopic=gethelp`
- `https://www.tibia.com/charactertrade/?subtopic=currentcharactertrades`
- `https://rubinot.com.br/`
- `https://rubinot.com.br/bazaar`
- `https://wiki.rubinot.com/en/`
- `https://tibia.fandom.com/wiki/`
- `https://tibiopedia.pl/`

No user-provided screenshots or personal/account data are stored in this audit. External references are capability evidence only. Oteryn implementation and availability claims must be proven from Oteryn code, controlled runtime evidence or an explicit server-data contract.

## Claim boundaries

- **Contract tested:** declared route/surface/state coverage passed its validator and acceptance evidence.
- **Repository proven:** the exact code/test/contract exists in Git at the identified SHA.
- **Staging-like proven:** an isolated production-like workflow passed for the exact tested SHA.
- **Product complete:** every capability classified as required is implemented with sufficient evidence; this is **not currently true**.
- **Production proven:** direct non-secret evidence from the exact deployed production release; this audit makes **no such claim**.
