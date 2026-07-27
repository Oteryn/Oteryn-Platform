# Oteryn Portal Completeness Matrix

## Purpose

This matrix is the authoritative route-by-route inventory for Issue #240. It replaces representative-surface reasoning with explicit evidence for every approved launch surface and meaningful state.

A route, controller, view or passing unit test is not sufficient to mark a row complete. Staging-verifiable rows require browser evidence against the exact running Synology Platform image SHA.

## Classification

- `COMPLETE` — product purpose, data behavior, security, localization, responsive/accessibility behavior, automated coverage and exact-SHA staging behavior are directly proven.
- `PARTIAL` — useful behavior exists but one or more launch-required capabilities or evidence dimensions are missing.
- `SKELETON` — route/view exists but exposes only a technical shell, placeholder or materially incomplete user workflow.
- `BROKEN` — behavior is incorrect, misleading, inaccessible, unsafe or unusable.
- `DEPLOYMENT_DRIFT` — observed deployed behavior cannot be reconciled with the release identity or repository source claimed for that environment.
- `DEFERRED_WITH_PRODUCT_DECISION` — capability is intentionally excluded from launch through an explicit product/contract decision and the UI communicates that truthfully.
- `UNKNOWN_PENDING_AUDIT` — temporary working state only; Issue #240 cannot close while any row remains unknown.

## Evidence dimensions

Every final row must record:

1. route and source ownership;
2. user purpose and expected actions;
3. data source, privacy and freshness semantics;
4. empty, validation, conflict, unavailable and authorization states;
5. desktop, tablet and mobile behavior;
6. keyboard, focus, landmarks, labels and contrast;
7. English and Polish behavior;
8. authorization, CSRF, rate limiting and output safety where applicable;
9. focused feature/integration/E2E coverage;
10. exact running Synology image SHA and live browser result.

## P0 release and evidence integrity

| Surface / state | Known route / source | Provisional class | Proven finding | Missing evidence / required action |
|---|---|---:|---|---|
| Running Synology Platform release identity | deployment image labels, application SHA, health/readiness evidence | `DEPLOYMENT_DRIFT` | Operator-observed localized Character Profile renders fields not present in `resources/views/game/character.blade.php` on current `main` or recorded staging SHA `415aa3febd04c8d9c61082d4a7451352bf084013`. | Read exact running Platform image digest/tag and application SHA; compare deployed file content; redeploy exact trusted SHA if mismatched; rerun complete matrix. |
| Final public staging smoke scope | archived final staging workflow/evidence | `PARTIAL` | Final public smoke recorded six public assertions only: localized homepage, Wiki, EN/PL launch articles, sitemap and robots. | Replace with route/state-complete live acceptance covering public, Identity, Account, Character, Admin and failure states. |
| Visual/UX acceptance claim | `.github/workflows/acceptance-validation.yml`, `scripts/acceptance/visual-acceptance-core.js` | `PARTIAL` | Pull-request `critical` profile does not execute the full exploratory visual collector; collector omits multiple later modules. | Expand inventory and run full exact-head plus exact-SHA Synology collection before broad PASS claim. |

## Shared shell and presentation

| Surface / state | Route / ownership | Provisional class | Current evidence | Required audit / remediation |
|---|---|---:|---|---|
| Public homepage | `/`, PublicPortal | `UNKNOWN_PENDING_AUDIT` | Dynamic composition exists. | Audit realistic, empty, stale and unavailable data in EN/PL across desktop/tablet/mobile; verify CTA hierarchy and content density. |
| Desktop public navigation | shared public header | `PARTIAL` | Navigation intentionally wraps; operator/audit evidence shows crowded hierarchy risk. | Define durable information architecture, dropdown/disclosure behavior and active-state rules; test long PL labels. |
| Mobile public navigation | shared public header | `UNKNOWN_PENDING_AUDIT` | Accessible disclosure mechanism exists in code. | Verify full route discoverability, open/close keyboard behavior, scroll locking, focus return and signed-in actions. |
| Public footer | shared public footer | `PARTIAL` | Mobile layout collapses to a long single column. | Reduce duplication, group or disclose sections, verify legal/support discoverability and long PL text. |
| Obsolete design preview | `/design/home-v2`, PublicPortal | `BROKEN` | Public route remains registered although the production homepage is active. | Remove route or restrict it to a non-production/admin preview boundary. |
| Homepage template selector | not implemented | `SKELETON` | No safe administrator-owned setting exists. | Define allowed template registry, Platform-owned setting, audited exact permission, preview and rollback; never accept arbitrary public template names. |

## Public editorial and community surfaces

| Surface / state | Route / ownership | Provisional class | Required final states |
|---|---|---:|---|
| News list | `/news`, CMS | `UNKNOWN_PENDING_AUDIT` | populated, empty, pagination, dependency failure, EN/PL, long titles, mobile |
| News detail | `/news/{slug}`, CMS | `UNKNOWN_PENDING_AUDIT` | published, missing, unpublished, stale translation, long body, metadata, mobile |
| Announcement ticker | homepage component, Announcements | `UNKNOWN_PENDING_AUDIT` | none, one, multiple, long text, active boundaries, safe links, EN/PL |
| Announcement administration | `/admin/announcements/**` | `UNKNOWN_PENDING_AUDIT` | list/create/edit, validation, stale edit, scheduling, permission denial, audit |
| Event list | `/events`, Events | `UNKNOWN_PENDING_AUDIT` | upcoming, active, archived, cancelled, empty, EN/PL, pagination/density |
| Event detail | `/events/{slug}`, Events | `UNKNOWN_PENDING_AUDIT` | all public states, missing/unpublished, date/timezone clarity, long content |
| Event administration | `/admin/events/**` | `UNKNOWN_PENDING_AUDIT` | create/edit/status, stale conflict, exact publish permission, audit |
| Download Center | `/download/{platform?}`, Downloads | `UNKNOWN_PENDING_AUDIT` | no release, current release, multiple platforms, checksum copy/readability, invalid platform, external artifact failure |
| Download administration | `/admin/downloads/**` | `UNKNOWN_PENDING_AUDIT` | create/edit/publish, approved-host validation, translation status, audit |
| Beginner Guide | `/getting-started`, CMS/Support | `UNKNOWN_PENDING_AUDIT` | published/missing/unpublished/stale translation, complete first-login steps |
| Server Information | `/server-information`, CMS/Support | `UNKNOWN_PENDING_AUDIT` | truthful rates/mechanics, missing/unpublished, EN/PL |
| Support landing | `/support`, Support | `UNKNOWN_PENDING_AUDIT` | approved links, missing content, no dead destinations, mobile |
| Report-a-bug guidance | `/support/report-a-bug`, Support | `UNKNOWN_PENDING_AUDIT` | actionable reporting path, privacy guidance, approved destinations |
| Rules | `/rules`, CMS/Support | `UNKNOWN_PENDING_AUDIT` | published version, EN/PL completeness, discoverability |
| Terms | `/legal/terms`, CMS/Support | `UNKNOWN_PENDING_AUDIT` | version/effective date, publication lifecycle, EN/PL |
| Privacy | `/legal/privacy`, CMS/Support | `UNKNOWN_PENDING_AUDIT` | version/effective date, real data practices, EN/PL |
| Cookies | `/legal/cookies`, CMS/Support | `UNKNOWN_PENDING_AUDIT` | actual cookie behavior, version/effective date, EN/PL |
| Support/legal administration | `/admin/support-content/**` | `UNKNOWN_PENDING_AUDIT` | exact permission, versioning, translation, validation, audit |

## Wiki and editorial media

| Surface / state | Route / ownership | Provisional class | Required final states |
|---|---|---:|---|
| Wiki index | `/wiki`, Wiki | `UNKNOWN_PENDING_AUDIT` | featured/populated/empty/unavailable, EN/PL, navigation density |
| Wiki category | `/wiki/category/{slug}`, Wiki | `UNKNOWN_PENDING_AUDIT` | populated/empty/missing/stale translation, pagination |
| Wiki article | `/wiki/{slug}`, Wiki | `UNKNOWN_PENDING_AUDIT` | published/missing/unpublished/stale, TOC, tables, callouts, images, long content |
| Wiki search | `/wiki/search`, Wiki | `UNKNOWN_PENDING_AUDIT` | empty query, no results, many results, abuse/rate limit, locale isolation |
| Wiki administration | `/admin/wiki/**` | `UNKNOWN_PENDING_AUDIT` | dashboard, create/edit, lifecycle, categories, revision restore, stale conflict, exact permissions |
| Wiki signed preview | administrator signed route | `UNKNOWN_PENDING_AUDIT` | valid/expired/tampered signature, auth/MFA, noindex/no-store |
| Editorial media library | `/admin/media/**` or effective module routes | `UNKNOWN_PENDING_AUDIT` | list/upload/preview/delete, codec validation, size errors, referenced-delete denial, audit |
| Wiki public media bytes | Wiki media serving boundary | `UNKNOWN_PENDING_AUDIT` | published reference, unpublished denial, tampered/missing object, integrity mismatch, correct MIME/cache |

## Public game data

| Surface / state | Route / ownership | Provisional class | Proven finding / required final states |
|---|---|---:|---|
| Online list | `/online`, PublicGameData | `UNKNOWN_PENDING_AUDIT` | zero online, populated, pagination, stale session exclusion, SQL unavailable 503, EN/PL/mobile |
| Highscores | `/highscores`, PublicGameData | `PARTIAL` | Existing query exposes name, level and numeric vocation only; audit readable vocation labels, pagination, empty/unavailable states and mobile tables. |
| Servers | `/servers`, PublicGameData | `UNKNOWN_PENDING_AUDIT` | available/maintenance/offline/stale/unavailable, long messages, freshness, EN/PL/mobile |
| Character search | `/characters?name=...`, PublicGameData | `PARTIAL` | Exact-name redirect exists but no dedicated result or suggestion workflow. Audit blank/invalid/not-found/case/spacing behavior and user guidance. |
| Character profile | `/characters/{name}`, PublicGameData | `SKELETON` + `DEPLOYMENT_DRIFT` | Repository query selects only `id`, `name`, `level`, `vocation`; view renders level and numeric vocation ID. Operator-observed staging screen differs and is mostly empty. Define approved public profile fields, human-readable mappings, guild relationship, unavailable-field policy, SEO and responsive presentation after Canary contract verification. |
| Guild index | `/guilds`, PublicGameData | `UNKNOWN_PENDING_AUDIT` | populated/empty/pagination/search/discoverability/mobile |
| Guild detail | `/guilds/{name}`, PublicGameData | `UNKNOWN_PENDING_AUDIT` | metadata, owner, ranks/members, long MOTD, empty guild, pagination, missing/unavailable/mobile |

## Identity and account lifecycle

| Surface / state | Route / ownership | Provisional class | Proven finding / required final states |
|---|---|---:|---|
| Registration | `/register`, Identity | `UNKNOWN_PENDING_AUDIT` | valid, duplicate email, password rules, rate limit, provisioning outcome, mail policy, EN/PL/mobile |
| Login | `/login`, Identity | `UNKNOWN_PENDING_AUDIT` | valid, invalid, disabled, MFA redirect, rate limits, safe return target, EN/PL/mobile |
| Logout | `/logout`, Identity | `UNKNOWN_PENDING_AUDIT` | CSRF, session invalidation, multiple tabs, post-logout navigation |
| Password recovery request | `/forgot-password`, Identity | `UNKNOWN_PENDING_AUDIT` | enumeration resistance, delivery/unavailable mail, rate limits, EN/PL/mobile |
| Password reset | `/reset-password/{token}`, Identity | `UNKNOWN_PENDING_AUDIT` | valid/expired/used/tampered token, session revocation, validation |
| Password change | `/password/change`, Identity | `UNKNOWN_PENDING_AUDIT` | current-password failure, policy, session revocation, MFA interaction |
| MFA settings | `/mfa`, Identity | `PARTIAL` | QR-first flow exists; audit no-MFA/enrolling/enabled states, disable flow, recovery-code regeneration/visibility policy, EN/PL/mobile. |
| MFA challenge | `/mfa/challenge`, Identity | `UNKNOWN_PENDING_AUDIT` | TOTP/recovery, invalid/expired pending state, rate limit, replay, navigation |
| Account Overview / Center | `/account`, Accounts | `SKELETON` | Current view shows only provisioning status, email, MFA, security links and character-creation CTA. It does not provide a complete account-management information architecture or character overview. |
| Canary provisioning | account registration/retry path | `PARTIAL` | Ready/pending/recoverable/conflict/unavailable states exist. Audit truthful progress, support escalation, idempotent retry, exact live behavior and recovery UX. |
| Character list for owned account | Account Center/read model | `SKELETON` | Repository read model exposes no owned-character collection to Account Overview. Add bounded read through immutable ready binding and present useful character actions without unapproved mutations. |
| Character creation | `/account/characters/create`, Characters | `UNKNOWN_PENDING_AUDIT` | ready/not-ready, validation, quota, duplicate, cross-account conflict, concurrency, dependency failure, success destination, EN/PL/mobile |
| Character rename | none | `DEFERRED_WITH_PRODUCT_DECISION` | Explicitly unapproved by current Canary contract; UI must not imply availability. |
| Character deletion | none | `DEFERRED_WITH_PRODUCT_DECISION` | Explicitly unapproved by current Canary contract; UI must not imply availability. |
| Existing account claim/import | none | `DEFERRED_WITH_PRODUCT_DECISION` | Greenfield-only ownership is explicit; onboarding must communicate it. |
| Authoritative game login | cross-repository bridge | `DEFERRED_WITH_PRODUCT_DECISION` or launch `BROKEN` | Requires explicit launch-scope decision. If users must log into the game with Platform authority at launch, this becomes P0 until end-to-end proven. |

## Administrator and operations surfaces

| Surface / state | Route / ownership | Provisional class | Required final states |
|---|---|---:|---|
| Admin dashboard | `/admin`, Admin | `PARTIAL` | Current route is a static view. Audit useful navigation, role-aware modules, system/content warnings, mobile and empty state. |
| News administration | `/admin/news/**` | `UNKNOWN_PENDING_AUDIT` | list/create/edit/translation/publication/validation/stale/audit |
| Managed pages administration | `/admin/pages/**` | `UNKNOWN_PENDING_AUDIT` | list/create/edit/translation/publication/validation/stale/audit |
| Roles | `/admin/roles`, Admin/RBAC | `UNKNOWN_PENDING_AUDIT` | assignment/removal, final-admin protection, search/density, denial, audit |
| Audit log | `/admin/audit`, Audit | `UNKNOWN_PENDING_AUDIT` | pagination, filters/search need decision, safe metadata, no secrets, mobile |
| Permission-denied states | privileged routes | `UNKNOWN_PENDING_AUDIT` | anonymous redirect, no MFA, no permission, mixed permissions, 403 consistency |
| Operational health | `/health`, deployment | `PARTIAL` | Liveness exists; audit whether readiness/version identity is available safely to operators without public secret exposure. |

## Error, resilience and abuse states

| State | Provisional class | Required evidence |
|---|---:|---|
| 403 authorization denied | `UNKNOWN_PENDING_AUDIT` | public/Identity/Admin layouts, EN/PL, recovery navigation, no technical leakage |
| 404 not found/unpublished | `UNKNOWN_PENDING_AUDIT` | all route families, EN/PL, correct noindex and navigation |
| 409 stale/conflict | `UNKNOWN_PENDING_AUDIT` | Events, Announcements, Wiki and other optimistic-lock paths; preserve user input safely |
| 422 validation | `UNKNOWN_PENDING_AUDIT` | every form field, summary/focus behavior, EN/PL, mobile |
| 429 rate limit | `UNKNOWN_PENDING_AUDIT` | login, registration, recovery, MFA, search and character create; retry guidance |
| 500 unexpected failure | `UNKNOWN_PENDING_AUDIT` | generic safe error, request ID, no stack trace, logs correlated |
| 503 dependency unavailable | `PARTIAL` | some public dependency paths are tested; expand to every dependent module and exact live staging recovery |
| Empty datasets | `PARTIAL` | some representative states exist; add every list/card/module and realistic populated/max-content fixtures |
| Long and maximal content | `SKELETON` | current broad acceptance does not prove long PL labels, dense navigation, long Wiki/legal/event/news content or maximal tables. |

## Required child delivery order

1. **Release identity and deployment drift reconciliation** — prove exact running Synology Platform SHA/image and prevent stale/mismatched container claims.
2. **Account Center completion** — owned-character overview, truthful onboarding/provisioning steps, security/account information architecture and responsive acceptance.
3. **Character Profile completion** — contract-backed public fields, human-readable mappings, guild relationship and explicit unavailable semantics.
4. **Shared shell and template administration** — remove public design preview, add safe audited template registry/selector and remediate desktop/mobile navigation/footer.
5. **Identity lifecycle closure** — every success/failure/rate-limit/MFA/recovery state.
6. **Public editorial/community closure** — News, Events, Announcements, Downloads, Support/legal in EN/PL and dense/empty/failure states.
7. **Wiki/media closure** — public/admin/media/preview/revision/security and large-content states.
8. **Admin operations closure** — module dashboard, role-aware navigation, all management forms, conflicts and audit usability.
9. **Complete E2E/visual matrix and exact-SHA Synology acceptance** — no representative sampling.

## Current conclusion

The portal must not currently be described as complete. Directly proven classifications include `DEPLOYMENT_DRIFT`, `SKELETON` and `PARTIAL`; the remaining inventory is intentionally fail-closed as `UNKNOWN_PENDING_AUDIT` until direct evidence exists.