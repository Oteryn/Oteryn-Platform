# Oteryn Portal Completeness Matrix

## Purpose

This is the fail-closed inventory for Issue #240. Code existence is not completion. A launch-scope row becomes `COMPLETE` only after product behavior, security, localization, responsive/accessibility behavior, automated regression coverage and exact-SHA live staging behavior are proven.

## Classification

- `COMPLETE` — every required evidence dimension is proven.
- `PARTIAL` — useful behavior exists, but launch-required capability or evidence is missing.
- `SKELETON` — route/view exists but is materially incomplete.
- `BROKEN` — behavior is incorrect, misleading, unsafe, inaccessible or unusable.
- `DEFERRED_WITH_PRODUCT_DECISION` — explicitly excluded by an approved product/contract decision and communicated truthfully.
- `UNKNOWN_PENDING_AUDIT` — not yet inspected completely; Issue #240 cannot close with this state.

## Evidence required per row

Route/source owner; user purpose/actions; data source/privacy/freshness; populated/empty/validation/conflict/unavailable/denied states; desktop/tablet/mobile; keyboard/focus/labels/contrast; EN/PL; authorization/CSRF/rate limiting/output safety; focused automated coverage; exact deployed SHA and live browser evidence.

## Release evidence integrity

| Surface | State | Evidence / remaining action |
|---|---:|---|
| Running Synology release identity | `COMPLETE` for identity only | Sanitized live preflight run `30275482522` reports `deployed_release_sha=415aa3febd04c8d9c61082d4a7451352bf084013` and matching immutable Platform/Gateway image tags. The operator screenshot matches that sparse release; the defect is not deployment drift. |
| Existing final public smoke | `PARTIAL` | The archived smoke exercised only six public assertions. It does not prove Identity, Account, Character, Admin or most current public modules. |
| Current visual/UX claim | `PARTIAL` | PR `critical` acceptance does not execute the complete exploratory collector, and the collector predates several later modules. PR #241 owns the exhaustive acceptance-contract remediation. |

## Shared shell and presentation

| Surface | State | Required closure |
|---|---:|---|
| Homepage realistic/empty/stale/unavailable states | `UNKNOWN_PENDING_AUDIT` | EN/PL desktop/tablet/mobile, CTA hierarchy, long content and dependency behavior. |
| Desktop navigation | `PARTIAL` | Remove crowded wrapped hierarchy; prove long Polish labels and signed-in actions. |
| Mobile navigation | `UNKNOWN_PENDING_AUDIT` | Discoverability, keyboard disclosure, focus return, scroll behavior and account actions. |
| Footer | `PARTIAL` | Reduce long single-column duplication and prove support/legal discoverability. |
| Public `/design/home-v2` | `PARTIAL` | Removed in the current source slice; exact-head tests and exact-SHA staging proof remain. |
| Administrator homepage template selector | `SKELETON` | Issue #244: allowlisted registry, durable setting, MFA/exact permission, audit, signed no-store preview, stale protection and rollback. |

## Public game data

| Surface | State | Current result / required closure |
|---|---:|---|
| Online | `PARTIAL` | Readable vocation mapping added; prove zero/populated/pagination/503/stale exclusion/EN-PL/mobile. |
| Highscores | `PARTIAL` | Readable vocation mapping added; prove empty/pagination/dependency failure/EN-PL/mobile table behavior. |
| Servers | `UNKNOWN_PENDING_AUDIT` | Available/maintenance/offline/stale/unavailable, long messages and freshness semantics. |
| Character search | `PARTIAL` | Exact-name redirect exists; blank/invalid/not-found/case/spacing and guidance remain. |
| Character profile | `PARTIAL` | Current slice adds readable vocation, approved guild link/no-guild and improved layout. Exact-head and staging browser proof remain; unreviewed private fields stay excluded. |
| Guild index | `UNKNOWN_PENDING_AUDIT` | Populated/empty/pagination/search/discoverability/mobile. |
| Guild detail | `UNKNOWN_PENDING_AUDIT` | Metadata, owner, ranks/members, long MOTD, empty/missing/unavailable and pagination. |

## Identity, account and character lifecycle

| Surface | State | Required closure |
|---|---:|---|
| Registration | `UNKNOWN_PENDING_AUDIT` | Success, duplicate email, password policy, rate limit, provisioning result, EN/PL/mobile. |
| Login/logout | `UNKNOWN_PENDING_AUDIT` | Valid/invalid/MFA/disabled/rate-limit/safe return, CSRF logout and session invalidation. |
| Password recovery/reset/change | `UNKNOWN_PENDING_AUDIT` | Enumeration resistance, mail failure, token lifecycle, policy and session revocation. |
| MFA enrollment/settings/challenge/recovery | `PARTIAL` | Prove all lifecycle states, replay/rate limits, recovery-code policy and responsive behavior. |
| Account Center | `PARTIAL` | Current slice adds owned active characters, slot use and populated/empty/not-ready/unavailable/limit states. Exact-head and staging proof remain; future profile/account actions require explicit product decisions. |
| Canary provisioning | `PARTIAL` | Existing ready/pending/recoverable/conflict/unavailable states need complete browser/live evidence and support path verification. |
| Character creation | `UNKNOWN_PENDING_AUDIT` | Ready/not-ready, validation, duplicate, quota, races, dependency failure, success destination, EN/PL/mobile. |
| Character rename/delete | `DEFERRED_WITH_PRODUCT_DECISION` | Not authorized by the Canary contract; UI must not imply availability. |
| Existing account claim/import | `DEFERRED_WITH_PRODUCT_DECISION` | Greenfield-only ownership is the approved model. |
| Authoritative game login | `DEFERRED_WITH_PRODUCT_DECISION` or launch `BROKEN` | A launch-scope decision is mandatory. Platform-authoritative game login remains a separate cross-repository bridge. |

## Editorial, community and legal

Every row below is `UNKNOWN_PENDING_AUDIT` until populated/empty/unpublished/missing/validation/stale/conflict/unavailable, EN/PL, desktop/tablet/mobile and authorization evidence is complete:

| Family | Surfaces |
|---|---|
| News | list, detail, admin list/create/edit/translation/publication |
| Announcements | homepage composition and admin lifecycle/scheduling |
| Events | list/detail active/upcoming/archived/cancelled and admin lifecycle |
| Downloads | platform filters, release/artifact/checksum states and administration |
| Getting started/server information | published/missing/stale translation and truthful game facts |
| Support | landing, report-a-bug, approved contact destinations and privacy guidance |
| Legal | rules, terms, privacy, cookies, versions/effective dates and administration |

## Wiki and editorial media

All remain `UNKNOWN_PENDING_AUDIT` for the complete state matrix: Wiki index/category/article/search; administrator dashboard/category/article lifecycle; signed preview valid/expired/tampered; revisions/restore/stale conflict; media list/upload/preview/delete/reference denial; public media published/unpublished/missing/integrity/MIME/cache behavior.

## Administrator and operations

| Surface | State | Required closure |
|---|---:|---|
| Admin dashboard | `PARTIAL` | Role-aware useful navigation, content/system warnings, empty state and mobile. |
| News/pages/events/announcements/downloads/support/Wiki/media administration | `UNKNOWN_PENDING_AUDIT` | Every form lifecycle, exact permission, MFA, validation, stale conflict and audit event. |
| Role management | `UNKNOWN_PENDING_AUDIT` | Assignment/removal, final-admin protection, denial and audit. |
| Audit log | `UNKNOWN_PENDING_AUDIT` | Pagination, usable filtering decision, safe metadata and mobile density. |
| Health/release identity | `PARTIAL` | Liveness exists; safe operator-visible readiness/version strategy remains to be finalized. |

## Cross-cutting errors and abuse

403, 404, 409, 422, 429, 500 and 503; empty datasets; maximum content; slow/unavailable dependencies; long Polish text; keyboard-only operation; focus restoration; mobile overflow; noindex/no-store states all remain `UNKNOWN_PENDING_AUDIT` or `PARTIAL` until every relevant route family is evidenced.

## Required delivery order

1. Account Center and Character Profile source remediation — current task.
2. Merge/consume PR #241 exhaustive acceptance infrastructure without duplicating ownership.
3. Shared shell plus Issue #244 safe template administration.
4. Identity and complete account/character lifecycle.
5. Public game-data search/server/guild closure.
6. Editorial/community/support/legal closure.
7. Wiki/media closure.
8. Administrator operations closure.
9. Exact-head full CI, complete browser/visual matrix, exact-SHA Synology deployment and live acceptance.

## Current conclusion

The running staging release is identified, but the portal is not complete. Issue #240 stays open until no approved launch row remains `SKELETON`, `BROKEN` or `UNKNOWN_PENDING_AUDIT`, and every `PARTIAL` row is either completed or explicitly reclassified through a truthful product decision.
