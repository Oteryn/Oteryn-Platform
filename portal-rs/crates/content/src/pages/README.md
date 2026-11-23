# Pages

Public routes are available in Polish and English under `/pl` and `/en`:
`/pages/{slug}`, `/start`, `/rules`, `/faq`, `/faq/{slug}`, `/legal/terms`,
`/legal/privacy`, `/legal/cookies`, `/server-information`, `/help`, `/support`
and `/support/report-a-bug`.
The bug-report page contains editorial instructions; it does not submit tickets.
Editorial slugs cannot be served through the generic page route.

The onboarding migration publishes bilingual starting editions of the setup,
rules, server information, support and bug reporting pages, plus three FAQ
entries. It preserves existing page slugs, including drafts. Staff can update
these editions from `/admin/pages`. To add a question at `/faq/{slug}`, use
`faq-{slug}` as its CMS page slug, the question as its title, and the answer
as its Markdown body. The FAQ index lists only published current editions.
The support pages link to the existing account-only ticket flow.

Legal content starts empty until an operator installs the legal defaults,
a host operator publishes a legal edition, or staff publish a page. Missing, draft,
future-dated, or stale translated content returns 404 on generic routes.
Reserved routes show an unpublished notice or,
for Help, Support and Report a Bug, localized support links. Database failures
return 503. English is the source locale. Saving a
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
published editions and exact versions at form display and again inside the
registration transaction. Live signup requires both languages for terms and
privacy. A localized pause page links to the public policies while signup is
unavailable. The acceptance stores the snapshot's page, locale,
and version so later language changes preserve the edition that was accepted.
Earlier consent records keep their original version without an inferred locale.
Live signup requires immutable, explicitly approved editions and rejects known
fictional operator values, including after a version is renamed. It checks
rendered text and image descriptions and validates the checksum of labeled NIP
numbers. Missing, stale,
mismatched or unapproved editions make signup unavailable and log the locale
and reason.

To enable live signup, replace the fictional operator details and publish
final editions under new versions. The Platform operator supplies and approves
the actual terms of service and privacy policy in both languages, including the
operator details and each document's effective date. Use the same version for
English and Polish of each document. Select **Approve this new legal edition as
final** when publishing each required edition, saving English before Polish.
Set the two signup version variables to the exact published values. Each
approval belongs to the immutable edition, including its locale and content.
An unchanged resave keeps the existing approval. Previously published
unapproved editions require a new version for approval.
In the staff panel, **Pages** shows live readiness for the four required
editions against the current signup configuration. Run
`oteryn-portal check-signup-legal` with the candidate's live configuration
before activating it. This read-only check requires both locales. See the
[Oracle deployment procedure](../../../../../deploy/oracle/README.md) for upgrades
from a release whose snapshots lack explicit approval.

In the qualification environment, open `/en/register` and `/pl/register`.
Check that each consent link opens the approved edition in the same language
and displays the configured version. Submit a fresh registration in each
locale and verify that the consent records reference those immutable editions
with the accepted locale and version. A submitted stale version must return a
validation error. Recheck readiness after changing the configured versions.

Host root can publish a final edition through the same save transaction as the
staff editor. The command uses the portal's database configuration in every
supported `OTERYN_ENV`. The actor must be an active, email-verified account
with `admin.access` and `cms.pages.manage`. Browser MFA cannot apply to a
host-root CLI. The transaction revalidates both permissions and records the
actor in `cms.page.saved`.

```sh
oteryn-portal publish-legal-edition --actor "$actor" \
  --slug terms-of-service --locale en --title 'Terms of Service' \
  --body-file ./terms.en.md --version "$version" \
  --effective "$effective" --final-approved
```

Supported slugs are `terms-of-service`, `privacy-policy`, and `cookie-policy`.
Supported locales are `en` and `pl`. Each UTF-8 Markdown file must contain
the real operator details. A literal `{{operator}}` anywhere in the body is
refused, and the final approval checks also reject known placeholders in
rendered content. The command
publishes immediately. Publish English first, then Polish for each slug.
An edition's title, body, effective date and approval remain immutable under
its locale and version.

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
