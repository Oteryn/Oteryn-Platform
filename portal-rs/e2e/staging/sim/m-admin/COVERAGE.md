# Admin and community staff route coverage

Source baseline `7847cf7de91957d4317e7693d31d049b8f1c5f8e`. The binary composes these routers after calling the web crate's router, and installs browser chrome and public middleware after merging them. Community exposes a staff party moderation router and a public party board that supplies its listings and reports. The browser-facing admin scope includes the separate operations console and the content, wiki, world, support and codes routes mounted by the binary. [Router composition](../../../../bin/oteryn-portal/src/main.rs#L252) · [staff support mount](../../../../bin/oteryn-portal/src/main.rs#L376) · [browser middleware](../../../../bin/oteryn-portal/src/main.rs#L486)

Role and permission management share `/admin/roles`. Accounts are reached through the player views. The catalog defines `portal.settings.manage`; the mounted routers have no separate browser route for site settings. The analytics ingestion and admin-control agent endpoints under `/internal/v1/` use machine identities and are outside the browser actor matrix. [RBAC catalog](../../../../crates/admin/src/rbac/catalog.rs#L177) · [analytics ingestion](../../../../crates/admin/src/analytics/mod.rs#L67) · [agent routing](../../../../crates/admin/src/control/http.rs#L182)

## Gate and role key

Staff rows require an active verified account, a validated browser session with MFA, `admin.access` and the route-specific permission. The admin host gate returns 404 before resolving the session when the panel is off or the host differs. A mounted standard guard redirects signed-out GETs to `/login` and returns 403 for insufficient privileges. Unsafe methods also require the ordinary browser form token. The rights in the inventory describe the source with the applicable gates open. Game-operation rights also depend on game permission configuration and a channel for operations that query the game. [Staff guard](../../../../crates/admin/src/web.rs#L296) · [role grants](../../../../crates/admin/src/rbac/catalog.rs#L312)

| Gate | Additional requirement |
| --- | --- |
| Panel | `OTERYN_ADMIN_PANEL=true` and the configured `OTERYN_ADMIN_HOST` |
| Players | Panel and `OTERYN_ADMIN_PLAYERS`. Character details and coin controls have feature prerequisites |
| Console | Panel and `OTERYN_ADMIN_CONSOLE` |
| Control | Panel and `OTERYN_ADMIN_CONTROL`. Approval eligibility and reauthentication are operation dependent |
| Analytics | Panel and `OTERYN_ADMIN_ANALYTICS_PRESENT`. View rights are `AdminAnalyticsView` or `GameAnalyticsRead` |
| Tickets | Panel and `OTERYN_SUPPORT_TICKETS` |
| Reports | Panel and `OTERYN_REPORTS_ENABLED` |
| Commerce | Panel and `OTERYN_ADMIN_COMMERCE`. Commands have premium, wallet, payment or refund prerequisites |
| Bazaar | Panel and `OTERYN_BAZAAR_MODERATION` |
| World | Panel and `OTERYN_WORLD_ADMIN` |
| Codes | Panel and `OTERYN_STORE_CODES_ENABLED`. Issuance depends on the value type being enabled |
| Party | `PORTAL_PARTY_ENABLED` |
| Staff party | Panel and `PORTAL_PARTY_ENABLED`. Staff also need `ManageSupportReports` |

PA = platform-admin, SA = security-admin, CE = content-editor, S = support, M = moderator, Ops = operations, GV = game-viewer, GS = game-support, GMS = game-master-senior, GO = game-operator, GA = game-admin, GDBA = game-dba, GAn = game-analyst, GSec = game-security, GR = game-release. “All staff” includes all 15. Staff outside each grant list are the denied role set. Party ownership is specific to the character or listing. Game access and moderation rights tied to a particular operation, approval or state still apply within each eligible role.

Permission names used in the inventory

| Routes | Permission beyond `admin.access` |
| --- | --- |
| Roles, console access and console approvals | `ManageRoles` |
| Platform players | `ManageSupportTickets` for search, detail, notes, session logout, identity attestation and reset requests. `KickAccountSessions` for kick and kick-only detail. `ManageRoles` for second-party reset approval. `ReleaseCoinPurchaseHold` for the hold action |
| Console players | `GamePlayerRead` for search and dossier. The empty view also admits `ManageSupportTickets` |
| Console audit | `ViewAudit` for platform audit or `GameAuditRead` for the game ledger |
| Operations console | `GameOpsRead`, `GameWorldRead`, `GameNodeRead` or `GameDbRead` for the respective operation. The unfiltered landing can show an unavailable section under `admin.access` |
| Staff support, reports, enforcement and party moderation | `ManageSupportTickets`, `ManageSupportReports`, `ManageSupportEnforcement` and `ManageSupportReports`, respectively |
| Commerce and Bazaar | `ManagePremiumTime` for grant/revoke, `ManageMarketplace` for adjust and Bazaar, `ReconcilePayments` for reconciliation/refund |
| World | `GameCatalogAccess` and `ViewGameCatalogSnapshots` plus `ActivateGameCatalogSnapshots`, `ManageGameCatalogProfiles` or `ManageGameCatalogTranslations` for the corresponding action |
| Store codes | `GenerateStoreCodes` |
| Pages, news, media, events and announcements | `ManagePages`, `ManageNews`, `ManageMedia`, `ManageEvents` and `ManagePortalAnnouncements`, respectively. Event publication also checks `PublishEvents` |
| Wiki | `WikiAccess` with `ManageWikiArticles` for editing and history. Restoration and publication check `PublishWiki` |

## Inventory

The table lists explicit router methods. Axum also answers HEAD through the corresponding GET registrations. Path parameters are shown in braces. State-changing browser forms carry `_csrf` through the normal browser form protection. Fields in the table are the route-specific inputs.

| Method | Path | Eligible staff or player | Gate | Fields | Registration |
| --- | --- | --- | --- | --- | --- |
| GET | `/admin` | All staff | Panel | Query: after (UUID) | [Router](../../../../crates/admin/src/web.rs#L254) |
| GET | `/admin/roles` | PA, SA | Panel | Query: after (UUID) | [Router](../../../../crates/admin/src/web.rs#L255) |
| POST | `/admin/roles` | PA, SA | Panel | action, account, role, key, permissions. Actions: create, set-permissions, grant, revoke | [Router](../../../../crates/admin/src/web.rs#L255) |
| GET | `/admin/players` | PA, S | Players | Query: q | [Router](../../../../crates/admin/src/players/http.rs#L69) |
| GET | `/admin/players/{id}` | PA, S, M (kick view only) | Players | Query: request, coin_hold, evidence. PA/S detail or M kick view | [Router](../../../../crates/admin/src/players/http.rs#L70) |
| POST | `/admin/players/{id}` | PA, S, M (kick only), SA (approve-reset only) | Players | action, body, reason, evidence, request, hold, ticket. Actions: note, logout, kick, release-coin-hold, verify-identity, request-reset, approve-reset | [Router](../../../../crates/admin/src/players/http.rs#L70) |
| GET | `/admin/console/players` | GS, GMS. PA and S (GET landing only) | Console | Query: q, character, lang | [Router](../../../../crates/admin/src/console/access.rs#L441) |
| POST | `/admin/console/players` | GS, GMS | Console | q, lang | [Router](../../../../crates/admin/src/console/access.rs#L441) |
| GET | `/admin/console/audit` | PA, SA, GV, GS, GMS, GO, GA, GSec | Console | Query: lang, request_id, admin, operation, target | [Router](../../../../crates/admin/src/console/access.rs#L445) |
| GET | `/admin/console/access` | PA, SA | Console | Query: lang | [Router](../../../../crates/admin/src/console/access.rs#L431) |
| GET | `/admin/console/approvals` | PA, SA | Console | Query: lang | [Router](../../../../crates/admin/src/console/access.rs#L435) |
| GET | `/console/dashboard` | All staff (landing). GV, GS, GMS, GO, GA (game data) | Console | Query: agent, target, request, lang | [Router](../../../../crates/admin/src/console/mod.rs#L393) |
| POST | `/console/dashboard` | GV, GS, GMS, GO, GA | Console | operation, agent, target, lang. Operations: ops.overview.get | [Router](../../../../crates/admin/src/console/mod.rs#L393) |
| GET | `/console/worlds` | All staff (landing). GV, GS, GMS, GO, GA (game data) | Console | Query: agent, target, request, lang | [Router](../../../../crates/admin/src/console/mod.rs#L397) |
| POST | `/console/worlds` | GV, GS, GMS, GO, GA | Console | operation, agent, target, lang. Operations: world.list, world.get | [Router](../../../../crates/admin/src/console/mod.rs#L397) |
| GET | `/console/nodes` | All staff (landing). GV, GS, GMS, GO, GA (game data) | Console | Query: agent, target, request, lang | [Router](../../../../crates/admin/src/console/mod.rs#L401) |
| POST | `/console/nodes` | GV, GS, GMS, GO, GA | Console | operation, agent, target, lang. Operations: node.list, node.get | [Router](../../../../crates/admin/src/console/mod.rs#L401) |
| GET | `/console/database` | All staff (landing). GV, GS, GMS, GO, GA, GDBA (game data) | Console | Query: agent, target, request, lang | [Router](../../../../crates/admin/src/console/mod.rs#L405) |
| POST | `/console/database` | GV, GS, GMS, GO, GA, GDBA | Console | operation, agent, target, lang. Operations: db.status, db.migrations.list | [Router](../../../../crates/admin/src/console/mod.rs#L405) |
| GET | `/admin/control/step-up` | All staff with an eligible request for their operation | Control | — | [Router](../../../../crates/admin/src/control/controls_http.rs#L42) |
| POST | `/admin/control/step-up` | All staff with an eligible request for their operation | Control | Query: request_id. Form: code (TOTP) | [Router](../../../../crates/admin/src/control/controls_http.rs#L42) |
| GET | `/admin/control/approvals/{id}` | Eligible approver for the pending operation | Control | id (path) | [Router](../../../../crates/admin/src/control/controls_http.rs#L46) |
| POST | `/admin/control/approvals/{id}` | Eligible approver for the pending operation | Control | id (path), confirmation form | [Router](../../../../crates/admin/src/control/controls_http.rs#L46) |
| GET | `/admin/analytics` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/data` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/creatures` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/creatures/data` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/activity` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/activity/data` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/levels` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/levels/data` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/health` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/health/data` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/economy` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/analytics/economy/data` | PA, GV, GS, GMS, GO, GA, GAn, GSec | Analytics | Query: world, family, start, end, from, to | [Router](../../../../crates/admin/src/analytics/web.rs#L121) |
| GET | `/admin/support/tickets` | PA, S | Tickets | Query: page | [Router](../../../../crates/admin/src/support_web.rs#L105) |
| GET | `/admin/support/tickets/{id}` | PA, S | Tickets | Query: after (message UUID) | [Router](../../../../crates/admin/src/support_web.rs#L122) |
| POST | `/admin/support/tickets/{id}` | PA, S | Tickets | lock_version, action. reply → body, internal. ticket_status → status | [Router](../../../../crates/admin/src/support_web.rs#L122) |
| GET | `/admin/moderation/reports` | PA, S | Reports | Query: page | [Router](../../../../crates/admin/src/support_web.rs#L105) |
| GET | `/admin/moderation/reports/{id}` | PA, S | Reports | Query: after (message UUID) | [Router](../../../../crates/admin/src/support_web.rs#L122) |
| POST | `/admin/moderation/reports/{id}` | PA, S | Reports | lock_version, action. moderate → status, public_outcome, moderator_notes | [Router](../../../../crates/admin/src/support_web.rs#L122) |
| GET | `/admin/moderation/enforcement` | PA, M | Panel | Query: page | [Router](../../../../crates/admin/src/support_web.rs#L105) |
| GET | `/admin/moderation/enforcement/create` | PA, M | Panel | — | [Router](../../../../crates/admin/src/support_web.rs#L117) |
| GET | `/admin/moderation/enforcement/{id}` | PA, M | Panel | Query: after (message UUID) | [Router](../../../../crates/admin/src/support_web.rs#L122) |
| POST | `/admin/moderation/enforcement/{id}` | PA, M | Panel | lock_version, action. sanction → create, status, target, category, public_reason, moderator_notes, effective_at, expires_at. appeal → appeal_status, appeal_outcome | [Router](../../../../crates/admin/src/support_web.rs#L122) |
| GET | `/{lang}/admin/support/tickets` | PA, S | Tickets | Query: page | [Router](../../../../crates/support/src/tickets/web.rs#L128) |
| GET | `/{lang}/admin/support/tickets/{id}` | PA, S | Tickets | Query: after (reply UUID) | [Router](../../../../crates/support/src/tickets/web.rs#L129) |
| POST | `/{lang}/admin/support/tickets/{id}/reply` | PA, S | Tickets | lock_version, body, internal | [Router](../../../../crates/support/src/tickets/web.rs#L130) |
| POST | `/{lang}/admin/support/tickets/{id}/status` | PA, S | Tickets | lock_version, status | [Router](../../../../crates/support/src/tickets/web.rs#L134) |
| GET | `/admin/commerce` | PA, M | Commerce | Query: after, lang | [Router](../../../../crates/admin/src/commerce/http.rs#L54) |
| POST | `/admin/commerce/{action}` | PA. M (adjust only) | Commerce | request_id, target, reason, days, amount. Actions: grant, revoke, adjust, reconcile, refund | [Router](../../../../crates/admin/src/commerce/http.rs#L55) |
| POST | `/admin/commerce/approvals/{id}` | PA. M (eligible adjust only) | Commerce | id (path). Pending approval from another eligible actor | [Router](../../../../crates/admin/src/commerce/http.rs#L56) |
| POST | `/admin/commerce/cancellations/{id}` | PA. M (eligible adjust only) | Commerce | id (path). Pending request and eligible actor | [Router](../../../../crates/admin/src/commerce/http.rs#L57) |
| GET | `/admin/{lang}/bazaar` | PA, M | Bazaar | Query: pending_after | [Router](../../../../crates/admin/src/bazaar.rs#L174) |
| POST | `/admin/{lang}/bazaar` | PA, M | Bazaar | action, id, listing, reason, coin_bps, gold_bps, listing_fee, promotion_price, world, family, production_key, revision_ref, minimum_value, accepted | [Router](../../../../crates/admin/src/bazaar.rs#L174) |
| GET | `/admin/world` | PA, Ops | World | Query: version, compare, page | [Router](../../../../crates/world/src/admin.rs#L77) |
| POST | `/admin/world` | PA, Ops | World | action, version, kind, key, allowed, pl, en. Actions: publish, visibility, names | [Router](../../../../crates/world/src/admin.rs#L77) |
| GET | `/{lang}/admin/store/codes` | PA | Codes | — | [Router](../../../../bin/oteryn-portal/src/code_routes.rs#L126) |
| POST | `/{lang}/admin/store/codes` | PA | Codes | request_id, kind, amount, count, max_per_code, max_per_account, valid_from, valid_until, note | [Router](../../../../bin/oteryn-portal/src/code_routes.rs#L126) |
| GET | `/admin/{lang}/party` | PA, S | Staff party | — | [Router](../../../../crates/community/src/party.rs#L273) |
| POST | `/admin/{lang}/party/{id}/moderate` | PA, S | Staff party | report_ids (comma-separated UUIDs), action, reason | [Router](../../../../crates/community/src/party.rs#L274) |
| GET | `/{lang}/party` | Public | Party | Query: world, purpose, level, profession | [Router](../../../../crates/community/src/party.rs#L288) |
| POST | `/{lang}/party` | Active owner of eligible character | Party | character, purpose, min_level, max_level, profession, starts_at, expires_at, note | [Router](../../../../crates/community/src/party.rs#L288) |
| POST | `/{lang}/party/{id}/report` | Eligible active player other than posting owner | Party | reason | [Router](../../../../crates/community/src/party.rs#L289) |
| POST | `/{lang}/party/{id}/remove` | Posting owner | Party | — | [Router](../../../../crates/community/src/party.rs#L290) |
| GET | `/admin/pages` | PA, CE | Panel | Query: slug, locale, page | [Router](../../../../crates/content/src/pages/staff.rs#L43) |
| GET | `/admin/pages/{slug}/{locale}` | PA, CE | Panel | slug, locale (path) | [Router](../../../../crates/content/src/pages/staff.rs#L44) |
| POST | `/admin/pages/{slug}/{locale}` | PA, CE | Panel | expected_revision, reviewed_source_revision, title, body, published_at, legal_version, legal_effective_date, final_approved, action | [Router](../../../../crates/content/src/pages/staff.rs#L44) |
| GET | `/admin/news` | PA, CE | Panel | Query: lang, page | [Router](../../../../crates/content/src/news.rs#L138) |
| POST | `/admin/news` | PA, CE | Panel | expected_revision, slug, title, body, published_at, title_pl, body_pl, translation_published_at | [Router](../../../../crates/content/src/news.rs#L138) |
| GET | `/admin/news/create` | PA, CE | Panel | Query: lang, page | [Router](../../../../crates/content/src/news.rs#L139) |
| GET | `/admin/news/{id}/edit` | PA, CE | Panel | Query: lang, page | [Router](../../../../crates/content/src/news.rs#L140) |
| POST | `/admin/news/{id}` | PA, CE | Panel | expected_revision, slug, title, body, published_at, title_pl, body_pl, translation_published_at | [Router](../../../../crates/content/src/news.rs#L141) |
| PUT | `/admin/news/{id}` | PA, CE | Panel | expected_revision, slug, title, body, published_at, title_pl, body_pl, translation_published_at | [Router](../../../../crates/content/src/news.rs#L141) |
| DELETE | `/admin/news/{id}` | PA, CE | Panel | expected_revision | [Router](../../../../crates/content/src/news.rs#L141) |
| POST | `/admin/news/{id}/delete` | PA, CE | Panel | expected_revision | [Router](../../../../crates/content/src/news.rs#L142) |
| GET | `/admin/media` | PA, CE | Panel | Query: page, lang | [Router](../../../../crates/content/src/media.rs#L123) |
| POST | `/admin/media` | PA, CE | Panel | Multipart: image, alt_text | [Router](../../../../crates/content/src/media.rs#L123) |
| GET | `/admin/media/{id}/content` | PA, CE | Panel | — | [Router](../../../../crates/content/src/media.rs#L124) |
| GET | `/admin/media/{id}/thumbnail` | PA, CE | Panel | — | [Router](../../../../crates/content/src/media.rs#L125) |
| DELETE | `/admin/media/{id}` | PA, CE | Panel | — | [Router](../../../../crates/content/src/media.rs#L126) |
| POST | `/admin/media/{id}/delete` | PA, CE | Panel | — | [Router](../../../../crates/content/src/media.rs#L127) |
| GET | `/admin/events` | PA, CE | Panel | Query: page | [Router](../../../../crates/content/src/events/web.rs#L206) |
| POST | `/admin/events` | PA, CE | Panel | reason, starts_at, ends_at, featured, en_title, en_slug, en_summary, en_body, pl_title, pl_slug, pl_summary, pl_body, lock_version | [Router](../../../../crates/content/src/events/web.rs#L206) |
| GET | `/admin/events/create` | PA, CE | Panel | — | [Router](../../../../crates/content/src/events/web.rs#L207) |
| GET | `/admin/events/{id}/edit` | PA, CE | Panel | — | [Router](../../../../crates/content/src/events/web.rs#L208) |
| PUT | `/admin/events/{id}` | PA, CE | Panel | reason, starts_at, ends_at, featured, en_title, en_slug, en_summary, en_body, pl_title, pl_slug, pl_summary, pl_body, lock_version | [Router](../../../../crates/content/src/events/web.rs#L209) |
| POST | `/admin/events/{id}` | PA, CE | Panel | reason, starts_at, ends_at, featured, en_title, en_slug, en_summary, en_body, pl_title, pl_slug, pl_summary, pl_body, lock_version | [Router](../../../../crates/content/src/events/web.rs#L209) |
| PUT | `/admin/events/{id}/status` | PA, CE | Panel | status, reason, lock_version | [Router](../../../../crates/content/src/events/web.rs#L213) |
| POST | `/admin/events/{id}/status` | PA, CE | Panel | status, reason, lock_version | [Router](../../../../crates/content/src/events/web.rs#L213) |
| GET | `/admin/announcements` | PA, CE | Panel | Query: page | [Router](../../../../crates/content/src/announcements_staff.rs#L41) |
| POST | `/admin/announcements` | PA, CE | Panel | lock_version, publication_state, severity, starts_at, ends_at, title, body, action_label, action_url, title_pl, body_pl, action_label_pl, translation_published_at | [Router](../../../../crates/content/src/announcements_staff.rs#L41) |
| GET | `/admin/announcements/create` | PA, CE | Panel | — | [Router](../../../../crates/content/src/announcements_staff.rs#L42) |
| GET | `/admin/announcements/{id}/edit` | PA, CE | Panel | id (path) | [Router](../../../../crates/content/src/announcements_staff.rs#L43) |
| POST | `/admin/announcements/{id}/edit` | PA, CE | Panel | lock_version, publication_state, severity, starts_at, ends_at, title, body, action_label, action_url, title_pl, body_pl, action_label_pl, translation_published_at | [Router](../../../../crates/content/src/announcements_staff.rs#L43) |
| GET | `/admin/wiki` | PA, CE | Panel | — | [Router](../../../../crates/wiki/src/web.rs#L69) |
| GET | `/admin/wiki/articles/create` | PA, CE | Panel | — | [Router](../../../../crates/wiki/src/web.rs#L70) |
| GET | `/admin/wiki/articles` | PA, CE | Panel | — | [Router](../../../../crates/wiki/src/web.rs#L71) |
| POST | `/admin/wiki/articles` | PA, CE | Panel | pl_title, pl_slug, pl_summary, pl_markdown, en_title, en_slug, en_summary, en_markdown, note, lock_version, entity_key | [Router](../../../../crates/wiki/src/web.rs#L71) |
| GET | `/admin/wiki/articles/{id}/edit` | PA, CE | Panel | — | [Router](../../../../crates/wiki/src/web.rs#L72) |
| POST | `/admin/wiki/articles/{id}` | PA, CE | Panel | pl_title, pl_slug, pl_summary, pl_markdown, en_title, en_slug, en_summary, en_markdown, note, lock_version, entity_key | [Router](../../../../crates/wiki/src/web.rs#L73) |
| GET | `/admin/wiki/articles/{id}/revisions` | PA, CE | Panel | Query: before (version) | [Router](../../../../crates/wiki/src/web.rs#L74) |
| GET | `/admin/wiki/articles/{id}/preview/{locale}` | PA, CE | Panel | — | [Router](../../../../crates/wiki/src/web.rs#L75) |
| POST | `/admin/wiki/articles/{id}/revisions/{source}/restore` | PA, CE | Panel | source (integer revision, path), lock_version | [Router](../../../../crates/wiki/src/web.rs#L79) |
| POST | `/admin/wiki/articles/{id}/{action}` | PA, CE | Panel | lock_version. Actions: submit-review, return-draft, publish, unpublish, archive | [Router](../../../../crates/wiki/src/web.rs#L83) |

## Route × actor evidence

A = signed out. P = fresh authenticated player. O = owning player. P2 = another player. The staff column names eligible staff and groups other roles as “other staff.” P, O, P2 and all staff are blocked for every row. Staging returns 404 with Access headers on this hostname, and registration and Mailpit did not supply authenticated actors. Ownership applies to party listings, characters, and accounts used as test targets. Owning an account or listing does not grant any staff permission.

The expected columns show the status on a suitable fixture with the relevant flags enabled and ordinary form protection satisfied where applicable. Staff with insufficient permissions and players on staff routes expect 403. The observed column is the actual signed-out staging response for all four projects. A 404 on a parameterized sample alone could mean the record is absent. The companion list and top-level checks also returned 404. Each group is a Playwright scenario prefixed `unavailable on staging:`. The checks used empty forms and a synthetic UUID for ID-bearing paths. They establish the deployed boundary and do not exercise form handlers or prove persisted effects.

| Method and route | A expected enabled | P / O / P2 expected enabled | Staff expected enabled | Observed on staging | Test group |
| --- | --- | --- | --- | --- | --- |
| GET `/admin` | 303 → /login | 403 | All staff: 200 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| GET `/admin/roles` | 303 → /login | 403 | PA, SA: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| POST `/admin/roles` | 303 → /login (with valid form token) | 403 | PA, SA: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| GET `/admin/players` | 303 → /login | 403 | PA, S: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| GET `/admin/players/{id}` | 303 → /login | 403 | PA, S, M (kick view only): 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| POST `/admin/players/{id}` | 303 → /login (with valid form token) | 403 | PA, S, M (kick only), SA (approve-reset only): 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| GET `/admin/console/players` | 303 → /login | 403 | GS, GMS. PA and S (GET landing only): 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| POST `/admin/console/players` | 303 → /login (with valid form token) | 403 | GS, GMS: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| GET `/admin/console/audit` | 303 → /login | 403 | PA, SA, GV, GS, GMS, GO, GA, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| GET `/admin/console/access` | 303 → /login | 403 | PA, SA: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| GET `/admin/console/approvals` | 303 → /login | 403 | PA, SA: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | panel, roles, players and access console |
| GET `/console/dashboard` | 303 → /login | 403 | All staff (landing). GV, GS, GMS, GO, GA (game data): 200 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| POST `/console/dashboard` | 303 → /login (with valid form token) | 403 | GV, GS, GMS, GO, GA: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| GET `/console/worlds` | 303 → /login | 403 | All staff (landing). GV, GS, GMS, GO, GA (game data): 200 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| POST `/console/worlds` | 303 → /login (with valid form token) | 403 | GV, GS, GMS, GO, GA: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| GET `/console/nodes` | 303 → /login | 403 | All staff (landing). GV, GS, GMS, GO, GA (game data): 200 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| POST `/console/nodes` | 303 → /login (with valid form token) | 403 | GV, GS, GMS, GO, GA: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| GET `/console/database` | 303 → /login | 403 | All staff (landing). GV, GS, GMS, GO, GA, GDBA (game data): 200 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| POST `/console/database` | 303 → /login (with valid form token) | 403 | GV, GS, GMS, GO, GA, GDBA: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| GET `/admin/control/step-up` | 303 → /login | 403 | All staff with an eligible request for their operation: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| POST `/admin/control/step-up` | 303 → /login (with valid form token) | 403 | All staff with an eligible request for their operation: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| GET `/admin/control/approvals/{id}` | 303 → /login | 403 | Eligible approver for the pending operation: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| POST `/admin/control/approvals/{id}` | 303 → /login (with valid form token) | 403 | Eligible approver for the pending operation: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | game operations console and staff controls |
| GET `/admin/analytics` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/data` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/creatures` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/creatures/data` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/activity` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/activity/data` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/levels` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/levels/data` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/health` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/health/data` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/economy` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/analytics/economy/data` | 403 | 403 | PA, GV, GS, GMS, GO, GA, GAn, GSec: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | analytics pages and data |
| GET `/admin/support/tickets` | 303 → /login | 403 | PA, S: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| GET `/admin/support/tickets/{id}` | 303 → /login | 403 | PA, S: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| POST `/admin/support/tickets/{id}` | 303 → /login (with valid form token) | 403 | PA, S: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| GET `/admin/moderation/reports` | 303 → /login | 403 | PA, S: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| GET `/admin/moderation/reports/{id}` | 303 → /login | 403 | PA, S: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| POST `/admin/moderation/reports/{id}` | 303 → /login (with valid form token) | 403 | PA, S: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| GET `/admin/moderation/enforcement` | 303 → /login | 403 | PA, M: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| GET `/admin/moderation/enforcement/create` | 303 → /login | 403 | PA, M: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| GET `/admin/moderation/enforcement/{id}` | 303 → /login | 403 | PA, M: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| POST `/admin/moderation/enforcement/{id}` | 303 → /login (with valid form token) | 403 | PA, M: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| GET `/{lang}/admin/support/tickets` | 303 → /login | 403 | PA, S: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| GET `/{lang}/admin/support/tickets/{id}` | 303 → /login | 403 | PA, S: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| POST `/{lang}/admin/support/tickets/{id}/reply` | 303 → /login (with valid form token) | 403 | PA, S: 302. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| POST `/{lang}/admin/support/tickets/{id}/status` | 303 → /login (with valid form token) | 403 | PA, S: 302. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | staff support, reports and enforcement |
| GET `/admin/commerce` | 303 → /login | 403 | PA, M: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | commerce, Bazaar, world and store codes |
| POST `/admin/commerce/{action}` | 303 → /login (with valid form token) | 403 | PA. M (adjust only): 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | commerce, Bazaar, world and store codes |
| POST `/admin/commerce/approvals/{id}` | 303 → /login (with valid form token) | 403 | PA. M (eligible adjust only): 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | commerce, Bazaar, world and store codes |
| POST `/admin/commerce/cancellations/{id}` | 303 → /login (with valid form token) | 403 | PA. M (eligible adjust only): 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | commerce, Bazaar, world and store codes |
| GET `/admin/{lang}/bazaar` | 303 → /login | 403 | PA, M: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | commerce, Bazaar, world and store codes |
| POST `/admin/{lang}/bazaar` | 303 → /login (with valid form token) | 403 | PA, M: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | commerce, Bazaar, world and store codes |
| GET `/admin/world` | 303 → /login | 403 | PA, Ops: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | commerce, Bazaar, world and store codes |
| POST `/admin/world` | 303 → /login (with valid form token) | 403 | PA, Ops: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | commerce, Bazaar, world and store codes |
| GET `/{lang}/admin/store/codes` | 303 → /login | 403 | PA: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | commerce, Bazaar, world and store codes |
| POST `/{lang}/admin/store/codes` | 303 → /login (with valid form token) | 403 | PA: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | commerce, Bazaar, world and store codes |
| GET `/admin/{lang}/party` | 303 → /login | 403 | PA, S: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | party board and staff moderation |
| POST `/admin/{lang}/party/{id}/moderate` | 303 → /login (with valid form token) | 403 | PA, S: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | party board and staff moderation |
| GET `/{lang}/party` | 200 | 200 all | 200 all | A: 404 × 4. P/O/P2/staff: blocked | party board and staff moderation |
| POST `/{lang}/party` | 403 | 303 with eligible owned character, 403 otherwise | Same ownership rule | A: 404 × 4. P/O/P2/staff: blocked | party board and staff moderation |
| POST `/{lang}/party/{id}/report` | 403 | 303 eligible P/P2, 403 O | Same ownership rule | A: 404 × 4. P/O/P2/staff: blocked | party board and staff moderation |
| POST `/{lang}/party/{id}/remove` | 403 | 303 O, 404 P/P2 | Same ownership rule | A: 404 × 4. P/O/P2/staff: blocked | party board and staff moderation |
| GET `/admin/pages` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/pages/{slug}/{locale}` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/pages/{slug}/{locale}` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/news` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/news` | 303 → /login (with valid form token) | 403 | PA, CE: 302. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/news/create` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/news/{id}/edit` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/news/{id}` | 303 → /login (with valid form token) | 403 | PA, CE: 302. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| PUT `/admin/news/{id}` | 303 → /login (with valid form token) | 403 | PA, CE: 302. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| DELETE `/admin/news/{id}` | 303 → /login (with valid form token) | 403 | PA, CE: 302. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/news/{id}/delete` | 303 → /login (with valid form token) | 403 | PA, CE: 302. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/media` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/media` | 303 → /login (with valid form token) | 403 | PA, CE: 302. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/media/{id}/content` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/media/{id}/thumbnail` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| DELETE `/admin/media/{id}` | 303 → /login (with valid form token) | 403 | PA, CE: 302. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/media/{id}/delete` | 303 → /login (with valid form token) | 403 | PA, CE: 302. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/events` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/events` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/events/create` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/events/{id}/edit` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| PUT `/admin/events/{id}` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/events/{id}` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| PUT `/admin/events/{id}/status` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/events/{id}/status` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/announcements` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/announcements` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/announcements/create` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/announcements/{id}/edit` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/announcements/{id}/edit` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/wiki` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/wiki/articles/create` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/wiki/articles` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/wiki/articles` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/wiki/articles/{id}/edit` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/wiki/articles/{id}` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/wiki/articles/{id}/revisions` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| GET `/admin/wiki/articles/{id}/preview/{locale}` | 303 → /login | 403 | PA, CE: 200. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/wiki/articles/{id}/revisions/{source}/restore` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
| POST `/admin/wiki/articles/{id}/{action}` | 303 → /login (with valid form token) | 403 | PA, CE: 303. Other staff: 403 | A: 404 × 4. P/O/P2/staff: blocked | admin content and wiki |
