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

After migrations, run `oteryn-portal install-legal-pages` with the portal's
migration database configuration. The live Oracle composition uses the
following command.

```sh
docker compose -p oteryn-portal -f /opt/oteryn-portal/compose.yml run --rm -T portal install-legal-pages
```

The installer publishes the paired Markdown files in `legal/` and records
the initial legal versions. It skips any slug
with locale content or publication history, including drafts and unpublished
documents. Rerunning it is safe. Replace the fictional operator block in
`legal/operator.md` and remove the draft notices before installing for launch.
If the drafts were already installed, publish replacements in the CMS under a
new legal version and save English before Polish. Coordinate the launch versions
with the versions recorded by account registration.

The editor is `/admin/pages`. Its create form opens
`/admin/pages/{slug}/{locale}`, which supports GET and CSRF-protected POST.
Actions are `save` and `delete`; an empty publication time keeps a draft.
Published legal documents require a version and effective date. Each published
locale/version snapshot is immutable, including at the database layer. Legal
pages can be unpublished but cannot be deleted through the editor.

Staff routes use T10B `configured_guard` and `guarded`, with exact admin-host checks, default-off panel flag, MFA,
`admin.access`, and `cms.pages.manage`. The configured adapter is T10B `PendingAuthentication` and
therefore denies access until authoritative authentication is integrated.
Tests inject an explicit staff adapter; no request field can grant staff access.
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
