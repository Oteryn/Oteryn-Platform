# Pages

Public routes are available in Polish and English under `/pl` and `/en`:
`/pages/{slug}`, `/rules`, `/legal/terms`, `/legal/privacy`, `/legal/cookies`,
`/server-information` and `/support/report-a-bug`.
The bug-report page contains editorial instructions; it does not submit tickets.
Editorial slugs cannot be served through the generic page route.

Content starts empty until an operator installs the legal defaults or staff
publish a page. Missing, draft, future-dated, or stale translated content
shows an unpublished notice on reserved routes and returns 404 on generic
routes. Database failures return 503. English is the source locale. Saving a
Polish translation binds it to the current English revision. Updating English
makes prior Polish revisions unavailable until reviewed and saved again.
Markdown is rendered and sanitized; titles and editor fields remain escaped.
Public responses are not cached, so unpublication has no stale response window.

After migrations, run the placeholder installer on a local or testing database.

```sh
OTERYN_ENV=local oteryn-portal install-legal-pages
```

The installer publishes the paired Markdown files in `legal/` and records
the initial legal versions. It preserves slugs with content or publication
history, including drafts and unpublished documents. Published placeholder
editions remain visible with a localized draft notice, including on live
profiles. The bundled fictional operator fields are in `legal/operator.md`.

Signup uses `OTERYN_TERMS_VERSION` and `OTERYN_PRIVACY_VERSION`. Both default
to `2026-11-05` in live or unconfigured operation and to the bundled draft
version under explicit local/testing configuration. The server checks the
published locale and exact versions at form display and again inside the
registration transaction. The acceptance stores the snapshot's page, locale,
and version so later language changes preserve the edition that was accepted.
Earlier consent records keep their original version without an inferred locale.
Live signup requires immutable, explicitly approved editions and rejects known
fictional operator values, including after a version is renamed. Missing, stale,
mismatched or unapproved editions make signup unavailable and log the locale
and reason.

To enable live signup, replace the fictional operator details and publish
final editions under new versions. Select **Approve this new legal edition as
final** when publishing each required edition, saving English before Polish.
Set the two signup version variables to the exact published values. Each
approval belongs to the immutable edition, including its locale and content.
An unchanged resave keeps the existing approval. Previously published
unapproved editions require a new version for approval.

The editor is `/admin/pages`. Its create form opens
`/admin/pages/{slug}/{locale}`, which supports GET and CSRF-protected POST.
Actions are `save` and `delete`; an empty publication time keeps a draft.
Published legal documents require a version and effective date. Each published
locale/version snapshot is immutable, including at the database layer. Legal
pages can be unpublished but cannot be deleted through the editor.

Staff routes use T10B `configured_guard` and `guarded`, with exact admin-host checks, default-off panel flag, MFA,
`admin.access`, and `cms.pages.manage`. The configured guard authenticates
staff through the server-side `PortalIdentity` adapter. Tests inject an
explicit staff adapter.
Writes and their before/after audit records commit in one transaction.

Behavior references: `legacy/app/Cms/PublicPageQuery.php`,
`legacy/app/Cms/Editorial/EditorialPageQuery.php`,
`legacy/app/Http/Controllers/Cms/PublicPageController.php`,
`legacy/app/Support/PublicEditorialPage.php`, and `legacy/app/Cms/Actions/SaveManagedPage.php`.
The contracts directory has no dedicated page HTTP contract; this mechanic has
no game producer or consumer and uses no game fixtures.

Tests read `PORTAL_TEST_DATABASE_URL`, print a skip notice when absent, and use
an isolated PostgreSQL schema. The database test covers publication, missing and
stale translations, scheduling, sanitized output, legal version refusal, staff
RBAC refusal, editor rendering, CSRF refusal, and a successful browser save.
