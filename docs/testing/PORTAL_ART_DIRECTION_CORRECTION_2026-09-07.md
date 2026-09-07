# Oteryn portal — reference-aligned art direction

Issue #1297 · PR #1298 · `feat/20260906-premium-portal-redesign`.

## Owner scope and authority

The owner supplied three cinematic Oteryn portal concepts and requested that their visual quality apply to the entire player-facing website, not only the homepage. The prior functional redesign is a foundation, not visual acceptance. The subsequent current-conversation instruction **“masz to wdrozyc na main”** authorizes protected-main integration after verification. It does not authorize production deployment, protection bypass, live identity/payment operations or backend contract changes.

The disposable execution handoff `docs/agents/prompts/OTERYN-PORTAL-ART-DIRECTED-REDESIGN-ASTRA.md` is retired in this implementation rather than installed as an unregistered permanent prompt. Its original contents remain in commit `341e5021499f3b437a4985409e4792436529a504`. The live Issue, PR and task packet govern continuation.

## Implemented candidate scope

- Shared `portal-art-direction.css` is loaded by public, identity and error layouts; the administrator layout retains its separate style system.
- Gold/ivory actions, blue-charcoal materials, stronger wordmark/navigation, reading hierarchy, contextual menus, form controls, ledgers, status/empty/error surfaces, account/security sections, library, downloads, Bazaar and support share the same visual language.
- The homepage has a composed world hero, genuine view-model world status in the first viewport, an image-led journal, six real discovery destinations, preserved character search and announcements/events, and the existing player onboarding path.
- News index/detail use decorative scenery alongside the existing escaped titles, dates and text. No reference-image date, population, availability or product claim is substituted for data.
- Routes, controllers, authorization, CSRF, field names, payment effects, database schemas and deployment workflows are unchanged.

The complete page-family/route ledger remains `docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md`. A shared style does not itself prove every state of every route.

## Decorative asset provenance

`public/images/oteryn-vistas.webp` contains six scenery-only crops from the owner's supplied generated concept `ChatGPT Image 7 wrz 2026, 15_40_58 (1).png`. It is not a gameplay screenshot, public content record or application acceptance evidence. No navigation, labels, counters or screenshot UI is used as an interactive surface.

Source PNG SHA-256: `098096e6b5772b63ee4e0554d4128f44ca217cbb11e2cec4caaf6da94fa8376b`.

Crop rectangles are `(left, top, right, bottom)` in the 1536 × 1024 source:

| Scene | Rectangle |
|---|---|
| Explore | `(119, 790, 327, 906)` |
| Fight | `(407, 790, 613, 905)` |
| Market | `(693, 790, 899, 905)` |
| Community | `(977, 790, 1183, 905)` |
| Library | `(1260, 790, 1466, 905)` |
| Chronicles | `(720, 80, 1224, 363)` |

Each crop is fitted to 240 × 134, combined left-to-right, and encoded as WebP quality 67. Output: 1440 × 134, 24706 bytes, SHA-256 `a0add073a3a2a887dc53587c63dec1a864b09fb45accd011118108cf626c1545`. Existing citadel and wordmark assets remain in use. No external host, frontend package or font dependency is introduced.

## Focused regression and browser fixture repairs

`PortalArtDirectionTest` checks real public/identity/error responses, both home locales, shipped asset dependencies and administrator-style isolation. Existing functional/security assertions remain intact.

The visual browser suite now registers a unique synthetic identity through the real acceptance-only registration path before character creation. The former read-model-only ready fixture did not create the backing game account. Bazaar/MFA fixture identities are unique across worker restarts; retries remain zero. Support/legal negative cases declare only their exact expected 403/404 response and pathname through the existing diagnostic API, which still fails on unexpected errors or missing expected responses. No diagnostic guard, scenario or authorization assertion is removed.

## Verification boundary

Publication is not acceptance. Exact-head PHP/static/required checks and the real Laravel browser matrix must complete, and actual screenshots must be inspected before integration. The current task checkpoint and live PR attestation record matching candidate results. Historical `7b4d03ea` screenshots and checks do not certify this art-directed candidate.
