# Content and wiki route coverage

The source baseline is `7847cf7de91957d4317e7693d31d049b8f1c5f8e`. The portal binary composes the module routers with the shared web router at `portal-rs/bin/oteryn-portal/src/main.rs:249`, mounts the CMS at lines 301–314, extends wiki with entity routes at lines 350–353, mounts Today at line 360 and mounts wiki places at line 473. The navigation registry lives in `crates/content/src/navigation.rs` and `crates/wiki/src/navigation.rs`. The web legacy router also redirects old public URLs to Polish editions. Those compatibility redirects are outside the content and wiki module routers.

Paths below are relative to `https://staging.oteryn.com`. A locale is `pl` or `en`. An admin ID is a UUID unless the route explicitly takes a slug or revision number. Router `get()` provides implicit HEAD handling. The inventory groups language pairs and method aliases on the same row when their handler contract is shared.

## Public router inventory

| Method | Path | Query or path fields and source contract |
| --- | --- | --- |
| GET | `/news` | `lang`, `page` (1–100000). Default Polish. Published editions. `crates/content/src/news.rs:68,128,220`. |
| GET | `/news/{slug}` | `lang` and slug. Published translation or 404. `news.rs:243–261`. |
| GET | `/{lang}/news` | Locale and `page`. Same publication gate. Invalid locale 404. `news.rs:128–175`. |
| GET | `/{lang}/news/{slug}` | Locale and slug. Polish edition must be approved and current against the English source revision. `news.rs:250–261`. |
| GET | `/events` | 303 to `/pl/events`. `crates/content/src/events/web.rs:30`. |
| GET | `/events/{slug}` | Valid slug redirects to the Polish edition. Invalid slug is 404. `events/web.rs:31–39`. |
| GET | `/{locale}/events` | Locale, `page` unsigned, zero-based. `events/web.rs:121–156`. |
| GET | `/{locale}/events/{slug}` | Locale and slug. Public edition or 404. `events/web.rs:159–176`. |
| GET | `/{pl,en}/pages/{slug}` | Custom slug of up to 160 characters. Public revision must be published and any translation must match source revision. Missing, invalid, reserved or unpublished custom slug gives 404. `crates/content/src/pages.rs:80–139`. |
| GET | `/{pl,en}/rules` | Reserved `rules` edition. Unpublished placeholder gives 200. `pages.rs:85,118–164`. |
| GET | `/{pl,en}/server-information` | Reserved `server-information` edition. Unpublished placeholder gives 200. `pages.rs:92,118–164`. |
| GET | `/{pl,en}/help` | Reserved `support` edition or help links. `pages.rs:86,118–164`. |
| GET | `/{pl,en}/support` | Same reserved edition as `/help`. `pages.rs:87,118–164`. |
| GET | `/{pl,en}/support/report-a-bug` | Reserved `report-a-bug` edition or help links. `pages.rs:93,118–164`. |
| GET | `/{pl,en}/legal/terms` | `terms-of-service` edition. Publication requires a legal version and effective date. `pages.rs:88,118–139`. |
| GET | `/{pl,en}/legal/privacy` | `privacy-policy` edition with the same publication conditions. `pages.rs:89,118–139`. |
| GET | `/{pl,en}/legal/cookies` | `cookie-policy` edition with the same publication conditions. `pages.rs:90,118–139`. |
| GET | `/today` | Optional `lang`, Polish by default. Localized source cards. `crates/content/src/today.rs:216–244,267–305`. |
| GET | `/{lang}/today` | Path locale. Invalid locale 404. `today.rs:219–244`. |
| GET | `/announcements/ticker` | Optional `lang`. HTML fragment, empty when there are no active announcements. `today.rs:184–202,246–258`. |
| GET | `/{lang}/announcements/ticker` | Path locale. Same fragment rules, invalid locale 404. `today.rs:228–265`. |
| GET | `/wiki` | Header locale. Query `q`, `page` (1–1000), home guide listing. Wiki search budget applies. `crates/wiki/src/web.rs:35–47,175–220`. |
| GET | `/wiki/search` | Header locale. Same query, guide search. `wiki/src/web.rs:38,175–220`. |
| GET | `/wiki/articles/{slug}` | Header locale and slug. Published article or 404. `wiki/src/web.rs:39,228–248`. |
| GET | `/{locale}/wiki` | Path locale, `q`, `page`. Same home listing. `wiki/src/web.rs:40,175–220`. |
| GET | `/{locale}/wiki/search` | Path locale, `q`, `page`. Same guide search. `wiki/src/web.rs:41,175–220`. |
| GET | `/{locale}/wiki/articles/{slug}` | Path locale and slug. Published article or 404. `wiki/src/web.rs:42,228–248`. |
| GET | `/wiki/{kind}` | Header locale, `q`, `kind`, `category`, `classification`, `page`. Requires `PORTAL_WIKI_ENTITIES_ENABLED` and an active world snapshot/catalog. `crates/wiki/src/entities.rs:61–69,118–157,390–419,485–506`. |
| GET | `/wiki/{kind}/{key}` | Header locale, entity kind and key. Same flag and data requirement. `entities.rs:121,625–650`. |
| GET | `/{locale}/wiki/{kind}` | Path locale, same filters and data requirement. Entity middleware extends the wiki home and search when enabled. `entities.rs:122,509–550`. |
| GET | `/{locale}/wiki/{kind}/{key}` | Path locale, entity kind and key. Same flag and data requirement. `entities.rs:123,625–650`. |
| GET | `/{pl,en}/wiki/calculators` | `calculator`, `current`, `target`, `offline`, `remaining`, `family`, `dummy`, `total`, `members`. `PORTAL_CALCULATORS_ENABLED` controls the route. Experience and training also require configured world references. `crates/wiki/src/calculators/web.rs:40–80,93–143,206–266`. |
| GET | `/{locale}/wiki/places` | `purpose` (hunt/bestiary/boss), `sort` (experience/profit), `party` (1/2/4). `PORTAL_WIKI_PLACES_ENABLED` and qualified data for populated results. Invalid filters give 400 before availability. `crates/wiki/src/places/mod.rs:55–65,79–105,150–165`. |

All public routes accept anonymous readers, players and staff. Content and wiki mutation authority is staff-only and applies independently of player ownership. Entity kinds are `items`, `creatures`, `npcs`, `spells`, `quests`, `loot`, `achievements` and `areas` (`entities.rs:18–27`). The home announcement banner is middleware in `crates/content/src/announcements.rs`, with no additional route.

## Staff router inventory

All routes require an enabled `OTERYN_ADMIN_PANEL`, exact `OTERYN_ADMIN_HOST`, staff session with MFA and `Access` before module permissions. The host check returns 404 before session processing (`crates/admin/src/web.rs:212–233,294–319,349–410`). Mutations use the shared browser form/session guard. `ContentEditor` and `PlatformAdmin` have the permissions listed here. `Support` is an example of a role without them (`crates/admin/src/rbac/catalog.rs:310–375`).

| Method | Path | Permission | Fields and source |
| --- | --- | --- | --- |
| GET | `/admin/news` | `ManageNews` | `lang`, `page`. Staff listing. `crates/content/src/news.rs:137–154,271–303`. |
| POST | `/admin/news` | `ManageNews` | `slug`, `title`, `body`, `published_at`, `title_pl`, `body_pl`, `translation_published_at`, optional `expected_revision`, `_csrf`. `news.rs:90–123,399–404`. |
| GET | `/admin/news/create` | `ManageNews` | `lang`. New form. `news.rs:305–315`. |
| GET | `/admin/news/{id}/edit` | `ManageNews` | ID, `lang`. Persisted form with revision. `news.rs:317–336`. |
| POST, PUT | `/admin/news/{id}` | `ManageNews` | Same write fields, with an existing ID and current revision. `news.rs:407–413,427–522,649–664`. |
| DELETE | `/admin/news/{id}` | `ManageNews` | ID and `expected_revision`; form/session token. `news.rs:141,527–607,645–664`. |
| POST | `/admin/news/{id}/delete` | `ManageNews` | Same deletion via HTML form. `news.rs:142,395,527–607`. |
| GET | `/admin/pages` | `ManagePages` | `page`, optionally `slug` with `locale` to open the edit form. `crates/content/src/pages/staff.rs:41–45,115–161`. |
| GET | `/admin/pages/{slug}/{locale}` | `ManagePages` | Slug and locale. Form supports creating a new edition. `pages/staff.rs:174–237`. |
| POST | `/admin/pages/{slug}/{locale}` | `ManagePages` | `title`, `body`, `published_at`, `legal_version`, `legal_effective_date`, `expected_revision`, integer `reviewed_source_revision`, optional `final_approved=yes` for legal, `action=save/delete`, `_csrf`. Legal versions and approved text have extra checks. `pages/staff.rs:61–113,239–255,379–517`, `templates/admin/pages/edit.html:1`. |
| GET | `/admin/events` | `ManageEvents` | `page` unsigned, zero-based. `crates/content/src/events/web.rs:206,240–260`. |
| POST | `/admin/events` | `ManageEvents` | `starts_at`, `ends_at`, optional `featured=true`, `reason`, `{en,pl}_{title,slug,summary,body}`, optional integer `lock_version`, `_csrf`. `events/web.rs:263–331,450–478`. |
| GET | `/admin/events/create` | `ManageEvents` | Draft form. `events/web.rs:207,433–439`. |
| GET | `/admin/events/{id}/edit` | `ManageEvents` | Existing ID. Status form also needs `PublishEvents`. `events/web.rs:208,394–449`. |
| POST, PUT | `/admin/events/{id}` | `ManageEvents` and `PublishEvents` when updating a published event | Same event form with ID and `lock_version`. `events/web.rs:209–211,479–488`, `events/repository.rs:134–157`. |
| POST, PUT | `/admin/events/{id}/status` | `ManageEvents`, `PublishEvents` | `status`, `reason`, integer `lock_version`, `_csrf`. `events/web.rs:213–215,489–510`. |
| GET | `/admin/announcements` | `ManagePortalAnnouncements` | Staff listing. `crates/content/src/announcements_staff.rs:41,62–75`. |
| POST | `/admin/announcements` | `ManagePortalAnnouncements` | `publication_state`, `severity`, `starts_at`, optional `ends_at`, `title`, `body`, `action_label`, `action_url`, `title_pl`, `body_pl`, `action_label_pl`, `translation_published_at`, `lock_version`, `_csrf`. `announcements_staff.rs:77–90,173–216`. |
| GET | `/admin/announcements/create` | `ManagePortalAnnouncements` | New form. `announcements_staff.rs:42,111–137`. |
| GET | `/admin/announcements/{id}/edit` | `ManagePortalAnnouncements` | Existing ID. `announcements_staff.rs:43–45,139–171`. |
| POST | `/admin/announcements/{id}/edit` | `ManagePortalAnnouncements` | Same write fields plus existing ID and lock. `announcements_staff.rs:43–45,217–244`. |
| GET | `/admin/media` | `ManageMedia` | `lang`, `page`. Upload form and library. `crates/content/src/media.rs:123,200–207,245–279`. |
| POST | `/admin/media` | `ManageMedia` | Multipart `image`, `alt_text` (1–500), `_csrf`. Image formats JPEG/PNG/WebP and up to 8 MiB. `media.rs:21,39–96,304–336`. |
| GET | `/admin/media/{id}/content` | `ManageMedia` | Existing ID. Normalized image. `media.rs:124,409–454`. |
| GET | `/admin/media/{id}/thumbnail` | `ManageMedia` | Existing ID. Thumbnail image. `media.rs:125,416–454`. |
| DELETE | `/admin/media/{id}` | `ManageMedia` | Existing ID, form/session token. `media.rs:126,456–539`. |
| POST | `/admin/media/{id}/delete` | `ManageMedia` | Existing ID and `_csrf` via HTML form. `media.rs:127,207,456–539`. |
| GET | `/admin/wiki`, `/admin/wiki/articles` | `WikiAccess` | Article listing. `crates/wiki/src/web.rs:69–71,261–272`. |
| GET | `/admin/wiki/articles/create` | `WikiAccess`, `ManageWikiArticles` | Draft form. `wiki/src/web.rs:70,298–324`. |
| POST | `/admin/wiki/articles` | `WikiAccess`, `ManageWikiArticles` | `{en,pl}_{title,slug,summary,markdown}`, `note`, integer `lock_version`, `_csrf`. `wiki/src/web.rs:366–418`. |
| GET | `/admin/wiki/articles/{id}/edit` | `WikiAccess` | Article and bilingual editor. `wiki/src/web.rs:72,326–363`, `wiki/src/repository.rs:106–115`. |
| POST | `/admin/wiki/articles/{id}` | `WikiAccess`, `ManageWikiArticles` | Same bilingual fields with current `lock_version`. Optional `entity_key` binding. `wiki/src/web.rs:366–443`. |
| GET | `/admin/wiki/articles/{id}/revisions` | `WikiAccess` | Optional integer `before`. Revision history. `wiki/src/web.rs:500–544`. |
| GET | `/admin/wiki/articles/{id}/preview/{locale}` | `WikiAccess`, `ManageWikiArticles` | ID and edition locale. `wiki/src/web.rs:566–593`. |
| POST | `/admin/wiki/articles/{id}/revisions/{source}/restore` | `WikiAccess`, `PublishWiki` | ID, integer source revision, integer `lock_version`, `_csrf`. Saves a new draft. `wiki/src/web.rs:547–563`, `wiki/src/repository.rs:265–321`. |
| POST | `/admin/wiki/articles/{id}/{action}` | `WikiAccess`, `ManageWikiArticles` for `submit-review` and `return-draft`, `PublishWiki` for `publish`, `unpublish`, `archive` | ID, named action, integer `lock_version`, `_csrf`. State transitions and stale-edit conflicts. `wiki/src/web.rs:453–483`, `wiki/src/domain.rs:72–100`, `wiki/src/repository.rs:324–388`. |

The wiki role catalog includes `ManageWikiCategories`. The wiki router exposes no category mutation route. The `publish-legal-edition` operator command is outside the browser surface. The legal page editor above uses the normal staff form.

## Route × actor results

The combined anonymous cells give the code expectation followed by the staging observation. The four projects use the same paths for protected admin endpoints and the named project locale for public paths. **P** denotes fresh and second players. **E**, **A**, and **S** denote content editor, platform admin, and support. **Mail** means the signed-in result was blocked by signup availability or staff password recovery. **Host** means the admin host gate prevents an authenticated CMS result. On the configured admin host, anonymous GET requests redirect to login. Staff without MFA and authenticated actors missing the required permission receive 403. Public reader access is actor independent in the handlers. The tests observed it with an anonymous browser. Authenticated player coverage requires successful registration, email verification and login. The content editor received a reset link, but submitting the new password returned 503 in `Content editor recovery and CMS entry`. The CMS gate continued to return 404 before resolving staff identity.

| Route and method | Anonymous expected → observed | P expected → observed | E/A expected → observed | S expected → observed | Test |
| --- | --- | --- | --- | --- | --- |
| GET `/news`, `/{lang}/news` | 200 list and 422 page bounds → localized empty list and 422 | Same → Mail | Same → Mail | Same → Mail | News and events navigation, aliases and publication visibility |
| GET `/news/{slug}`, `/{lang}/news/{slug}` | Unpublished 404 → 404 | Same → Mail | Same → Mail | Same → Mail | News and events navigation, aliases and publication visibility |
| GET `/events`, `/events/{slug}` | Valid aliases 303 to Polish and invalid slug 404 → matched | Same → Mail | Same → Mail | Same → Mail | News and events navigation, aliases and publication visibility |
| GET `/{locale}/events` | 200 list and 400 bad page → localized empty list and 400 | Same → Mail | Same → Mail | Same → Mail | News and events navigation, aliases and publication visibility |
| GET `/{locale}/events/{slug}` | Unpublished or invalid 404 → 404 | Same → Mail | Same → Mail | Same → Mail | News and events navigation, aliases and publication visibility |
| GET `/{pl,en}/pages/{slug}` | Absent or rejected slug 404 → localized absent and 404 | Same → Mail | Same → Mail | Same → Mail | Editorial, help and legal pages remain readable across language and reload |
| GET `/{pl,en}/rules`, `/{pl,en}/server-information` | Reserved draft placeholder 200 → localized placeholder | Same → Mail | Same → Mail | Same → Mail | Editorial, help and legal pages remain readable across language and reload |
| GET `/{pl,en}/help`, `/{pl,en}/support`, `/{pl,en}/support/report-a-bug` | Reserved fallback 200 → localized heading | Same → Mail | Same → Mail | Same → Mail | Editorial, help and legal pages remain readable across language and reload |
| GET `/{pl,en}/legal/{terms,privacy,cookies}` | Published edition or reserved fallback 200 → localized editions | Same → Mail | Same → Mail | Same → Mail | Editorial, help and legal pages remain readable across language and reload |
| GET `/today`, `/{lang}/today` | 200 and unknown locale 404 → localized digest and 404 | Same → Mail | Same → Mail | Same → Mail | Today and the announcement ticker show localized publication state |
| GET `/announcements/ticker`, `/{lang}/announcements/ticker` | 200 fragment for en/pl, 404 for invalid locale → empty 200 and 404 | Same → Mail | Same → Mail | Same → Mail | Today and the announcement ticker show localized publication state |
| GET `/wiki`, `/wiki/search`, `/{locale}/wiki`, `/{locale}/wiki/search` | 200 and invalid 422 → localized results and validation | Same → Mail | Same → Mail | Same → Mail | Wiki search, history navigation and unavailable world data |
| GET `/wiki/articles/{slug}`, `/{locale}/wiki/articles/{slug}` | Missing 404 → localized 404 | Same → Mail | Same → Mail | Same → Mail | Wiki search, history navigation and unavailable world data |
| GET `/wiki/{kind}`, `/{locale}/wiki/{kind}` | Disabled 404, malformed query extractor 422 → matched for the sampled invalid input and all eight kinds | Same → Mail | Same → Mail | Same → Mail | Wiki search, history navigation and unavailable world data |
| GET `/wiki/{kind}/{key}`, `/{locale}/wiki/{kind}/{key}` | Disabled 404 → localized 404 for all eight kinds | Same → Mail | Same → Mail | Same → Mail | Wiki search, history navigation and unavailable world data |
| GET `/{pl,en}/wiki/calculators` | Disabled 404 → 404 with and without inputs | Same → Mail | Same → Mail | Same → Mail | Wiki search, history navigation and unavailable world data |
| GET `/{locale}/wiki/places` | Unavailable 503 and malformed filters 400 → localized unavailability and 400 | Same → Mail | Same → Mail | Same → Mail | Wiki search, history navigation and unavailable world data |
| GET `/admin/news` | Host conditional 404 → 404 | Host conditional 404 → Mail | List news → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/news` | Host conditional 404 → 404 | Host conditional 404 → Mail | Create a news draft or publication → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/news/create` | Host conditional 404 → 404 | Host conditional 404 → Mail | Open the draft form → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/news/{id}/edit` | Host conditional 404 → 404 | Host conditional 404 → Mail | Open a saved item → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST, PUT `/admin/news/{id}` | Host conditional 404 → 404 | Host conditional 404 → Mail | Update a news revision → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| DELETE `/admin/news/{id}` | Host conditional 404 → 404 | Host conditional 404 → Mail | Delete the current revision → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/news/{id}/delete` | Host conditional 404 → 404 | Host conditional 404 → Mail | Delete via browser form → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/pages` | Host conditional 404 → 404 | Host conditional 404 → Mail | List editions and open a selected slug → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/pages/{slug}/{locale}` | Host conditional 404 → 404 | Host conditional 404 → Mail | Open a custom or legal edition → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/pages/{slug}/{locale}` | Host conditional 404 → 404 | Host conditional 404 → Mail | Save or delete a custom or legal edition → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/events` | Host conditional 404 → 404 | Host conditional 404 → Mail | List events → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/events` | Host conditional 404 → 404 | Host conditional 404 → Mail | Create an event draft → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/events/create` | Host conditional 404 → 404 | Host conditional 404 → Mail | Open the draft form → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/events/{id}/edit` | Host conditional 404 → 404 | Host conditional 404 → Mail | Open a saved event → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST, PUT `/admin/events/{id}` | Host conditional 404 → 404 | Host conditional 404 → Mail | Update the current revision → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST, PUT `/admin/events/{id}/status` | Host conditional 404 → 404 | Host conditional 404 → Mail | Change publication state → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/announcements` | Host conditional 404 → 404 | Host conditional 404 → Mail | List announcements → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/announcements` | Host conditional 404 → 404 | Host conditional 404 → Mail | Save a new announcement → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/announcements/create` | Host conditional 404 → 404 | Host conditional 404 → Mail | Open the new form → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/announcements/{id}/edit` | Host conditional 404 → 404 | Host conditional 404 → Mail | Open a saved announcement → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/announcements/{id}/edit` | Host conditional 404 → 404 | Host conditional 404 → Mail | Update the current revision → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/media` | Host conditional 404 → 404 | Host conditional 404 → Mail | List assets and open upload form → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/media` | Host conditional 404 → 404 | Host conditional 404 → Mail | Upload a supported image → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/media/{id}/content` | Host conditional 404 → 404 | Host conditional 404 → Mail | Read normalized image → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/media/{id}/thumbnail` | Host conditional 404 → 404 | Host conditional 404 → Mail | Read thumbnail → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| DELETE `/admin/media/{id}` | Host conditional 404 → 404 | Host conditional 404 → Mail | Delete asset → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/media/{id}/delete` | Host conditional 404 → 404 | Host conditional 404 → Mail | Delete via browser form → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/wiki` | Host conditional 404 → 404 | Host conditional 404 → Mail | List articles → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/wiki/articles` | Host conditional 404 → 404 | Host conditional 404 → Mail | List articles → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/wiki/articles/create` | Host conditional 404 → 404 | Host conditional 404 → Mail | Open the bilingual draft form → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/wiki/articles` | Host conditional 404 → 404 | Host conditional 404 → Mail | Create a draft → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/wiki/articles/{id}/edit` | Host conditional 404 → 404 | Host conditional 404 → Mail | Open the saved revision → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/wiki/articles/{id}` | Host conditional 404 → 404 | Host conditional 404 → Mail | Edit the current draft or reviewed revision → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/wiki/articles/{id}/preview/{locale}` | Host conditional 404 → 404 | Host conditional 404 → Mail | Preview both editions → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| GET `/admin/wiki/articles/{id}/revisions` | Host conditional 404 → 404 | Host conditional 404 → Mail | List revisions → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/wiki/articles/{id}/revisions/{source}/restore` | Host conditional 404 → 404 | Host conditional 404 → Mail | Restore an earlier edition as a draft → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/wiki/articles/{id}/submit-review` | Host conditional 404 → 404 | Host conditional 404 → Mail | Move draft into review → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/wiki/articles/{id}/return-draft` | Host conditional 404 → 404 | Host conditional 404 → Mail | Return reviewed article to draft → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/wiki/articles/{id}/publish` | Host conditional 404 → 404 | Host conditional 404 → Mail | Publish reviewed revision → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/wiki/articles/{id}/unpublish` | Host conditional 404 → 404 | Host conditional 404 → Mail | Withdraw published revision → Host | Refusal → Host | CMS routes are unavailable on the staging host |
| POST `/admin/wiki/articles/{id}/archive` | Host conditional 404 → 404 | Host conditional 404 → Mail | Archive a live revision → Host | Refusal → Host | CMS routes are unavailable on the staging host |
