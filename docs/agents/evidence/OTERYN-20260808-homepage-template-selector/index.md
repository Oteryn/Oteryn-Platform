# OTERYN-20260808-homepage-template-selector evidence

## Implementation candidate

- Issue: #244
- Original audited base: `0582b0e853d1b5e983f664452268e7777c886904`
- Product implementation head: `5e186ca5b84c43f5bcc1b35a6a2d520bf86a3a14`
- Current-main synchronization merge: `f814f223556f3478ec57808259c40f8e34cbb341`
- Validation intensity: `HEIGHTENED`

## Static implementation review

- Public `/` resolves only through `HomepageTemplateRegistry`; stored values cannot become Blade paths.
- `production` remains the registry default and maps to the existing `home` view.
- `classic` maps to the bounded `home-classic` view derived from the previously reviewed classic portal presentation.
- Public `/design/*` routes remain absent.
- All selector routes require `auth`, `mfa.confirmed` and `portal.settings.manage`.
- Preview is GET-only and sends `X-Robots-Tag: noindex, nofollow` plus `Cache-Control: no-store, private, max-age=0`.
- Mutation uses a singleton row, `lockForUpdate`, optimistic `version`, an approved-key allowlist and bounded `AdminAuditRecorder` events.
- `AdminAuditRecorder` writes through the same database connection while activation/rollback is inside the transaction, so a failed transaction cannot commit a success audit independently of the setting mutation.
- Migration seeds `production` at version `0`, so migration alone does not change public presentation.
- Unknown stored active keys resolve to `production` and set an administrator-visible drift warning.
- No production role bundle, external repository, environment or production state is changed.

## Scope-amended audit support

Issue #244 originally classified `.github/workflows/**` as forbidden. The durable Issue #244 scope amendment recorded at `2026-08-08T09:16:00+02:00` explicitly superseded that restriction for **only** `.github/workflows/portal-exhaustive-audit.yml` and named the associated portal coverage/source-identity files as newly owned paths.

The workflow diff stays within that amendment:

- exact named-route count advances from `240` to proven `244`;
- exact classified-route count advances from `228` to proven `232`;
- exclusions remain exactly `12`;
- source-identity evidence includes the new delegated-binding, content-scale, media-state and selector coverage fragments/validators.

The amendment is backed by Portal Exhaustive Audit run `31245683153`, which had already proven `244/232/12` with `infrastructure_error_count: 0`; the stale workflow count assertion was the remaining failure. No other workflow is changed.

A later Issue #244 amendment at `2026-08-08T13:50:00+02:00` additionally authorized `scripts/acceptance/coverage/test-portal-media-state-evidence.mjs` after exact validation proved the deterministic strict fixture was using the non-strict repository-input loader while the strict validator correctly consumed all fragment inputs.

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

`tests/Feature/HomepageTemplates/HomepageTemplateMigrationTest.php` proves that the migration creates exactly one selector row with `production`, version `0`, no previous key and no activating administrator identity.

`scripts/acceptance/tests/homepage-template-selector.spec.mjs` runs with retries disabled and covers desktop/tablet/mobile overflow, private preview headers, keyboard activation, public render switch, Polish localization, keyboard rollback and restoration of the production template.

## Validation history

Release-candidate head `81690cf811b8d9b5590b5a8e9a5c616a436a6b3c` passed the repository-selected Agent Governance, Portal Exhaustive Audit, CI, Acceptance E2E and Visual UX, Content Scale Acceptance, Portal Acceptance Contract, Phase 7, Platform DB Outage Validation, Game Auth Ticket Concurrency, Edge Security Emulation and Build Synology Staging Images workflows. That head was later superseded only because protected `main` advanced through the unrelated Issue #908 contract repair/closeout.

A compare from the old PR base to protected `main@0c6c630ecc7cb55c3a7ee8eac4d2627a91b751ca` proved that intervening main changes were limited to the PublicGameData privacy contract and the archived #908 task, with no homepage-selector feature-path overlap. Those main changes were merged into this branch at `f814f223556f3478ec57808259c40f8e34cbb341` before final validation.

Recovery exact-head validation then exposed only checkpoint-metadata shape errors (`PASS_WITH_ONE_SUPERSEDED_PENDING` was not an accepted checkpoint result enum). The metadata was corrected without product-code changes. On subsequent head `42e0eba180bcdaaea99f915495253019dd2dbe9b`, Agent Governance, Portal Exhaustive Audit, CI, Content Scale Acceptance, Edge Security Emulation, Game Auth Ticket Concurrency, Platform DB Outage Validation and Build Synology Staging Images had already passed before this evidence-reconciliation commit superseded that head.

## Final exact-head requirement

Because this evidence correction creates a new release-candidate SHA, prior green workflow generations remain regression evidence only. Final delivery still requires on the **new exact head**:

- Agent Governance;
- repository-selected CI, including formatting, static analysis and PHPUnit;
- Portal Exhaustive Audit;
- applicable zero-retry browser/portal acceptance;
- any other required workflows selected by repository routing;
- final full-diff self-review with zero unresolved material findings;
- confirmation that protected `main` has not advanced incompatibly before merge.

No production activation is claimed or authorized by this repair.