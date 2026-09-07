# Oteryn Platform — cinematic redesign implementation and acceptance

Issue #1297; PR #1298; branch `feat/20260906-premium-portal-redesign`.

## Owner boundary

The owner supplied three cinematic Oteryn reference images and subsequently required completion of the whole Platform redesign and integration into protected `main`. This supersedes the earlier task-only no-merge/preview hold. It does not grant production deployment, protected-environment, live identity/payment, gate-bypass or cross-repository authority.

Admission: protected `main@721d560c18ef8a6eefbf4ccd685d8302985bbd3d`; resumed PR head `341e5021499f3b437a4985409e4792436529a504`.

## Coherent implementation

The existing Laravel/Blade application remains the product. The shared cinematic layer is included by public, identity, error and administrator layouts. The classic registered homepage already delegates to the real homepage. No detached demo, frontend framework, remote asset dependency or production configuration is introduced.

- Homepage: layered world art; Oteryn display hierarchy; genuine world-state panel; illustrated real news; six real discovery destinations; preserved character search, announcements, upcoming events and registration/download/guide path.
- Editorial and events: illustrated lead, smaller stories, readable long-form articles, calendar/date hierarchy and decorative event cover.
- Worlds/community: restrained world ledgers, readable status/freshness/maintenance, rankings, semantic scrollable tables, character/guild mastheads and rosters.
- Knowledge/catalog: taxonomy/search, reading/TOC hierarchy, restrained metadata and genuine existing catalogue media and unavailable states.
- Player workspace: owner-only character roster, account/security/MFA/recovery, payments presentation, Bazaar, session analysis and support conversations use the same materials and controls without changing any operation contract.
- Downloads/support/legal/errors: release/artifact hierarchy, explicit choices, calm prose/forms and consistent branded failure surfaces. Legal prose is not replaced by artwork.
- Administration: same Oteryn branding, task-focused sidebar/mobile navigation, dense readable forms and tables. The navigation now exposes existing events, announcements, downloads, editorial/legal and homepage-template routes in addition to retained destinations. Server-side RBAC/MFA and every mutation remain unchanged.

`PORTAL_REDESIGN_REVIEW_2026-09-06.md` remains the public/player route ledger (98 application HTML/navigation entries plus vendor OAuth consent). Shared templates are intentional coverage, not proof of browser acceptance. Administration adds real navigation-family/form captures; API/media bytes are not visual pages.

## Artwork and truthfulness

`PORTAL_RENDER_ASSET_PROVENANCE_2026-09-07.json` records the owner-provided source images and exact scenery-only crops. The six local WebP strips are decorative, not gameplay screenshots or claims about a particular news item. Rendered text/buttons were excluded from the crops. Existing citadel and wordmark assets are reused. No font files or third-party game assets were added.

Reference player numbers, server uptime, dates, event names and commerce promises were not copied into product data. Empty/stale/unavailable/maintenance and disabled capabilities continue to reflect existing view models.

## Validation changes and evidence boundary

The visual test preserves existing assertions and coverage. It now provisions its player through actual isolated Laravel registration before sign-in and character creation, instead of constructing a read-model-only binding to a nonexistent game account. Unique fixture identities avoid collisions on worker restart. It also checks that the art-direction stylesheet actually loads on every captured page and exercises administrator MFA before capturing privileged page families. Raw authentication traces/video/automatic screenshots remain disabled; sensitive generated values remain masked.

Support Legal declares exact counted expected 403/404 responses through the existing diagnostic guard. Denial, non-disclosure, publication, locale and audit assertions remain unchanged. The guard is not disabled. The unrelated payment acceptance file remains unchanged unless a new exact-head failure requires a separately inspected correction.

Local syntax/static inspection is supporting evidence only. The application requires PHP 8.5 and Composer dependencies unavailable in this network-isolated sandbox. Real browser evidence must come from the existing GitHub Actions Laravel runtime and must identify the published exact candidate. Historical green results and previous screenshots do not certify these new bytes.

## Acceptance status

Implementation prepared; current-candidate application, visual, accessibility/viewport and CI results are pending publication. This document does not claim the complete redesign is accepted or merged. Remaining work: publish one coherent candidate, fix observed failures, inspect real family screenshots and actor-to-result paths, pass required checks and integrate through protected repository controls under the owner's new authorization.

Rollback: revert the coherent presentation/assets/test diff. No database migration, route, controller, auth/payment/domain or production change.
