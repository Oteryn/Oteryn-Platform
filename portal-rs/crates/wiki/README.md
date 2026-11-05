# Editorial wiki

T13 owns editorial guides and optional entity annotations, not generated world
statistics or assets (T14b). `Repository` accepts trusted RBAC `CheckContext`s and
an authoritative `AccountStateReader`. Every write checks current explicit wiki
permissions and staff MFA. Audit records commit with the editorial mutation.

`web::staff_router` accepts the existing staff session and account-state adapters
and mounts through `oteryn_admin::web::guarded`; production mounts through
`configured_guard` with `Permission::WikiAccess`. Operation-specific manage and
publish checks remain in the repository.
The binary uses the same `PendingAuthentication` adapter as the staff shell on
this base branch: staff routes deny until the identity/MFA integration supplies
these adapters. `OTERYN_ADMIN_PANEL` defaults off, and an exact configured admin
host is required. Browser mutations use the shared session/CSRF middleware.
Never construct authentication context from request bodies or headers.

Public routes are `/wiki`, `/wiki/search`, `/wiki/articles/{slug}` with `/pl` and
`/en` variants. Search uses the existing PostgreSQL `wiki-search` limiter (30 per
minute per locale/source), with peer addresses from `ConnectInfo`, and a GIN
full-text index using the `simple` configuration for both languages. Results
contain only published, current revisions. Empty searches return no results;
browse lists published guides. Pages contain at most twelve results.

Both translations must be complete to publish. Saving or restoring a snapshot
returns the article to draft, so a new review submission is required. Published
articles must be unpublished before editing; archived articles are terminal.
Version checks and row locks reject stale writes. Slugs remain reserved in
history to avoid assigning an old article URL to unrelated content. Restore
copies the whole bilingual snapshot into a new revision; it never mutates history.
Database triggers forbid updates, deletes and truncation of revisions/reviews.

Markdown source rejects raw HTML and dangerous URL protocols. Every preview and
public render also uses a restricted HTML allow-list. Links follow the existing
portal boundary: root-relative paths, safe fragments, or HTTPS without userinfo.
Raw HTML and arbitrary images are omitted; editorial media integration belongs
to the media task. Templates escape source, titles and summaries normally; only
the sanitizer's output reaches the HTML-safe template slot.

Run `PORTAL_TEST_DATABASE_URL=... cargo test -p oteryn-wiki` with PostgreSQL 18.6.
The five database tests create and remove isolated schemas, and explicitly print
a skip notice if that variable is unset. No compile-time SQL query macros are used.
