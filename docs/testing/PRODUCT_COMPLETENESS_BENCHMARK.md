# Oteryn Product Completeness Benchmark

## Audit identity

- Parent issue: #268
- Repository baseline: `0a9a00014f55d7b2146d4ab151cd2a1b7c5bcd3d`
- Machine-readable ledger: `docs/testing/product-completeness-benchmark.json`
- Delivered-surface contract: `scripts/acceptance/coverage/portal-coverage-manifest.json` plus sorted fragments under `scripts/acceptance/coverage/surfaces/`
- Character Bazaar delivery: PR #270, merge `0f19656e0875d0a10b22002ac0e096deb20e94d8`
- Account-security lifecycle delivery: PR #283
- First Game Catalog delivery: PR #272
- Support/moderation lifecycle delivery: PR #293
- Community-data completeness delivery: PR #298
- First Game Catalog scope closure: PR #303 (PR #272 delivery evidence)
- Character profile preferences candidate: PR #308 / Issue #307

## Verdict

The current Oteryn delivered-surface contract is broad and its declared routes have exact repository and isolated staging-like acceptance evidence. That is not the same as product completeness against Tibia/RubinOT-style account, character, commerce, support, community-data and knowledge ecosystems.

The benchmark ledger contains **43 capabilities**:

| Delivery status | Count |
|---|---:|
| Implemented | 23 |
| Partial | 3 |
| Missing | 14 |
| Untested | 0 |
| Not applicable | 3 |

Relevance classification:

| Relevance | Count |
|---|---:|
| Required | 22 |
| Planned | 13 |
| Optional / differentiator | 5 |
| Not applicable | 3 |

**Oteryn must not claim benchmark product completeness while required partial or missing capabilities remain open.** Issue #307 delivers the Platform-owned comment, character privacy and optional main-character portion of #277; deletion/restore, rename, transfer and authoritative achievements remain open. Commerce is intentionally planned rather than part of the current non-commercial launch boundary, but #278 is mandatory before any commercial activation. Structured spell/NPC/quest/achievement expansion is tracked by #301, while optional maps and hunt/discovery planning is tracked by #302.

Issues #276, #279 and #280 now have delivered Platform-owned account-security, support/moderation and read-only community-data lifecycles for their approved boundaries. Issue #281's first authoritative Game Catalog scope was delivered by PR #272; closing that scope does not implement or authorize the #301/#302 follow-ups, Canary account unlink/rebind, native game-account deletion, character deletion or production deployment.

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
| Account overview and Canary provisioning | Covered | ready, pending, recoverable, conflict, missing, retry, internal identifiers hidden, per-character public-profile management and optional main-character state |
| Account security and lifecycle | Covered | EN/PL, confirmed email change and old-address recovery, cooldown, active-session inventory, targeted/current/all-other revocation, privacy, recovery-key issue/revoke/use/replay denial, termination grace/cancel/finalize, stale-session redirect |
| Character creation and visibility | Covered | validation, reserved/duplicate name, quota, ownership injection denial, idempotent outcome, public visibility |
| Public home and SEO | Covered | available, empty, stale, dependency unavailable, EN/PL, published-only sitemap/robots |
| Public news and managed pages | Covered | published, empty, not found, unpublished hidden, long content, EN/PL |
| Public game data | Covered for complete approved read model | categorized/vocation highscores, privacy-aware character profiles, guild directory/search/detail, deaths/kill statistics, online, servers, pagination, empty, not found, dependency unavailable/restored |
| Core admin, RBAC, CMS and audit | Covered | guest/no-MFA/no-permission denial, exact role assignment/removal, final-admin protection, publication and audit |
| Localization administration | Covered | EN/PL, missing/incomplete/draft/published/stale translation and route-preserving switch |
| Downloads | Covered | empty/current/filter, validation, draft/publish, URL denial, dependency recovery, audit, EN/PL |
| Events | Covered | empty/upcoming/active/archived/cancelled/not found, draft/publish, stale conflict, permission denial, audit, EN/PL |
| Announcements | Covered | no active/active/future/expired/draft, escaped text, localization staleness, conflict, permission and audit |
| Support and legal content | Covered | published/unpublished/missing, legal version, approved links, validation, permission and EN/PL |
| Authenticated support and moderation | Covered | owner-scoped tickets/reports/enforcement history, moderator queues, MFA/RBAC, notifications, retention, privacy, EN/PL and D/T/M |
| Public Wiki | Covered | home/category/article/search, empty/invalid search, not found, unavailable/recovery, EN/PL |
| Wiki administration | Covered | draft/review/publish/unpublish/archive, revision restore, stale conflict, signed preview, permission and audit |
| Game Catalog | Covered for first server-backed slice | immutable import, activation/rollback, active item/weapon/creature/loot projections, provenance, admin inspection and D/T/M |
| Editorial media administration | Covered | empty/upload/validation/integrity rejection/delete/reference protection/permission |
| Media and Wiki preview endpoints | Supporting endpoints | authorized, unreferenced, draft hidden, integrity failure, invalid signature, not found |
| Character Bazaar public catalogue/detail | Covered | active/filter/empty/detail/immutable snapshot/history/not found/EN/PL/private saga hidden |
| Character Bazaar account lifecycle | Covered | wallet/reservation, watch, validation, escrow, bid/outbid, buy-now, cancel/expiry, history, recovery, idempotency |
| Character Bazaar administration | Covered | MFA/permission denial, wallet lookup/adjustment, ledger/audit, empty recovery queue, bounded retry, ownership conflict |

The route contract proves that delivered surfaces are classified. It does not prove that absent character, commerce or knowledge capabilities are acceptable omissions.

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

Implemented:

- character creation with validation, quotas, ownership controls and public visibility;
- privacy-aware public character profile with skills, guild/rank, house, deaths, kill statistics, related characters and status;
- authenticated owner-editable Platform comments with bounded plain-text validation, escaped rendering and audit;
- per-character visibility that can narrow account association, status, guild, house, skills, deaths and kill statistics while account-level privacy remains the upper bound;
- optional single main-character selection serialized by Identity-row locking and proven by a real-MariaDB concurrent race;
- Character Bazaar ownership transfer through escrow, which is not a world-transfer or general owner-management service.

Required gaps:

- deletion grace period and restore;
- rename with history/cooldown and cross-surface consistency.

Planned gaps:

- world or channel transfer;
- achievement selection after an authoritative source exists.

Issue **#307** delivers the Platform-owned profile-preference portion of **#277**. Parent #277 remains open only for the mutation/achievement lifecycles above; no Canary write is authorized by this slice.

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

Implemented:

- authenticated owner-scoped ticket creation, listing, detail, replies and explicit close/reopen states;
- bounded player, content and guild reports with idempotency, pending limits, owner history and public-safe outcomes;
- exact-permission, confirmed-MFA administrator queues for tickets, reports and enforcement;
- Platform-owned account-visible warning, restriction and suspension history with acknowledgement and appeals;
- deterministic pending/sent/failed notification delivery state with failure isolation;
- bounded audit metadata, optimistic locking, configurable retention/pruning and privacy-safe EN/PL responsive presentation.

Canary remains authoritative for game-server bans. This lifecycle does not mutate Canary ban or account-status tables, and support attachments remain disabled until a separately reviewed secure upload model exists.

Issue **#279** is delivered for the approved Platform-owned boundary.

### Public and community data

Implemented:

- online players and server/channel status with dependency recovery states;
- Character Bazaar public/account/admin lifecycle;
- allowlisted highscore categories with vocation filtering and truthful global scope;
- privacy-aware complete public character profiles with guild, house, deaths and kill statistics;
- guild directory/search and public-safe detail;
- latest deaths with localized empty, unavailable and recovery states.

Explicit product/ownership decisions:

- guild administration remains outside Platform until Canary approves a least-privilege mutation contract;
- transfer history is not applicable until an authoritative transfer service exists;
- polls are not adopted for the current launch contract;
- public punishment publication is excluded; enforcement remains account-visible only.

Issue **#280** is delivered for the approved read-only boundary through merged PR #298 (`7533b12b1e1c6d266c6bf5a8800e584fad23a01e`).

### Knowledge and tooling ecosystem

The Wiki retains its public/editorial workflow, while PR #272 delivered the first authoritative versioned server-backed Game Catalog scope required by Issue #281.

Delivered first scope:

- immutable versioned snapshots with provenance, explicit activation and rollback;
- active-profile Oteryn items, weapons, creatures and exact visible loot/reverse-source relations;
- public EN/PL responsive reads and exact-permission confirmed-MFA administrator inspection;
- generated Canary artifact verification and MariaDB import, activation, candidate activation and rollback evidence.

Issue **#281** is complete for that first scope through PR #272. Closure does not promote unsupported capabilities or change production state.

Remaining planned expansion:

- structured spells, quests, NPCs and achievements only when additive authoritative producer/consumer contracts exist — **#301**;
- world-transfer documentation only when the owner-management and transfer service is defined — **#277**;
- complete historical introduction/removal, spawn, availability and provenance expansion as separately bounded contract work.

Optional differentiators and product discovery:

- maps and interactive maps;
- hunting-place discovery and calculators;
- equipment presets, Huntfinder-like matching and linked tasks;
- battle-pass and other server-specific engagement/system catalogues.

These optional/product-decision capabilities are tracked by **#302**. Third-party pages remain UX references only and are not Oteryn availability proof.

## Gap backlog and priority

| Priority boundary | Issue | Scope |
|---|---|---|
| Delivered account/security lifecycle | #276 | Confirmed email, sessions, privacy, recovery key, termination and explicit MFA/binding policy |
| Required remaining character lifecycle | #277 | Delete/restore, rename, authoritative achievements and controlled world/channel transfer contracts; profile editing/privacy/main selection are delivered by #307 |
| Delivered support/moderation lifecycle | #279 | Platform tickets, reports, enforcement history, notifications, retention and privacy |
| Delivered community-data completeness | #280 | Read-only rich profiles, guild directory/search/detail, highscore filters, deaths/statistics and explicit exclusions |
| Mandatory before commercial activation | #278 | Premium, coins, products, provider/webhook/refund/chargeback lifecycle |
| Delivered first server-backed Game Catalog scope | #281 | Versioned item/weapon/creature/loot catalogue delivered by PR #272; closeout preserves deferred boundaries |
| Planned structured catalogue expansion | #301 | Authoritative spells, NPCs, quests, achievements and exact cross-links |
| Optional knowledge/discovery planning | #302 | Maps, hunt tools, presets and server-specific discovery/product decisions |
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
