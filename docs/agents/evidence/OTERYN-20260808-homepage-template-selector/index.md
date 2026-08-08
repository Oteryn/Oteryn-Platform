# OTERYN-20260808-homepage-template-selector evidence

## Implementation candidate

- Issue: #244
- Base: `0582b0e853d1b5e983f664452268e7777c886904`
- Product implementation head: `5e186ca5b84c43f5bcc1b35a6a2d520bf86a3a14`
- Validation intensity: `HEIGHTENED`

## Static implementation review

- Public `/` resolves only through `HomepageTemplateRegistry`; stored values cannot become Blade paths.
- `production` remains the registry default and maps to the existing `home` view.
- `classic` maps to the bounded `home-classic` view derived from the previously reviewed classic portal presentation.
- Public `/design/*` routes remain absent.
- All selector routes require `auth`, `mfa.confirmed` and `portal.settings.manage`.
- Preview is GET-only and sends `X-Robots-Tag: noindex, nofollow` plus `Cache-Control: no-store, private, max-age=0`.
- Mutation uses a singleton row, `lockForUpdate`, optimistic `version`, an approved-key allowlist and bounded `AdminAuditRecorder` events.
- Migration seeds `production` at version `0`, so migration alone does not change public presentation.
- Unknown stored active keys resolve to `production` and set an administrator-visible drift warning.
- No role bundle, external repository, workflow, environment or production state is changed.

## Focused coverage added

`tests/Feature/HomepageTemplates/HomepageTemplateSelectorTest.php` covers:

- deterministic production default;
- removed public design gallery;
- authentication/MFA/exact-permission denial;
- private noindex/no-store preview;
- approved activation and public render switch;
- invalid key rejection;
- stale-version conflict;
- rollback and audit evidence;
- removed-key fallback and warning;
- Polish administrator localization.

## Pending exact-head evidence

- focused PHP test execution through repository CI;
- migration/rollback gate selected by changed paths;
- full required CI;
- applicable browser acceptance;
- exact-head full-diff self-review after CI evidence.
