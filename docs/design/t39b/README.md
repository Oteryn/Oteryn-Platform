# T39B owner review packet

Status: **pending owner approval**. This task is the static-mockup T39b row, not the duplicate home-page implementation row in §10. No runtime Rust, route, migration, feature flag or product translation service changes are needed: these documents cannot perform account, game or money operations. All such controls are disabled; no form submits, scripts or remote assets exist.

## Review entry points

Serve the repository root (`python3 -m http.server 8099 --bind 127.0.0.1`), then open the HTML links. All screens share the T39 dark tokens, components, system font, visible focus and a two-column desktop composition that stacks on phones. Existing earlier mockups remain unchanged.

| Screen | Polish | English | Decision requested |
| --- | --- | --- | --- |
| Home | [PL](../../../portal-rs/templates/mockups/t39b/home-pl.html) | [EN](../../../portal-rs/templates/mockups/t39b/home-en.html) | Onboarding first, news second, truthful world status |
| Player panel | [PL](../../../portal-rs/templates/mockups/t39b/account-pl.html) | [EN](../../../portal-rs/templates/mockups/t39b/account-en.html) | Characters before security and balances |
| Bazaar | [PL](../../../portal-rs/templates/mockups/t39b/bazaar-pl.html) | [EN](../../../portal-rs/templates/mockups/t39b/bazaar-en.html) | Filters, price/currency, escrow evidence and bid explanation |
| Atlas | [PL](../../../portal-rs/templates/mockups/t39b/atlas-pl.html) | [EN](../../../portal-rs/templates/mockups/t39b/atlas-en.html) | Text search/layers beside schematic map; stacked mobile panels |
| Journal | [PL](../../../portal-rs/templates/mockups/t39b/journal-pl.html) | [EN](../../../portal-rs/templates/mockups/t39b/journal-en.html) | Completed-session summary, price provenance, read-only inventory |
| Statistics | [PL](../../../portal-rs/templates/mockups/t39b/statistics-pl.html) | [EN](../../../portal-rs/templates/mockups/t39b/statistics-en.html) | Scope, privacy and accessible daily history before charts |

Screenshots are named `<screen>-<locale>-<360|1280>.png` alongside this document. Each HTML has success, refusal/unavailable and invalid-input disclosures. These are independent visual examples, not a state machine or executable validation. The review column explains the design decision; it is not intended for production pages.

## Governing sources and boundaries

- Plan §§1–4, 5.1, 5.5–5.9, 7, 8, 10 and D4/D11/D13/D15/D17/D19: six required compositions, privacy, trading safeguards, journal timing and map provenance.
- T39 is present in the supplied base (`static/css/tokens.css`, `components.css`, `static/design/README.md`). The dependency branch `origin/codex/portal-rs-t39` was absent at task start; no merge was necessary.
- `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`: unknown/stale is neither offline nor zero. Home deliberately renders unavailable rather than inventing current counts.
- `docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md` and `CHARACTER_PROFILE_PREFERENCES_CONTRACT.md`: game evidence and ownership precede private output; profile policy narrows disclosure. Statistics never expose hidden characters or online time without permission.
- `docs/contracts/PUBLIC_COMMUNITY_DATA_CONTRACT.md`: failed reads differ from empty data and use localized sanitized unavailable responses. No new status codes or response bodies are proposed here.
- Laravel reference: `routes/web.php` account route uses authentication; `app/Http/Controllers/Accounts/AccountOverviewController.php` rejects a missing Identity with 403, selects the configured native read model, and avoids leaking preference read failures. The mockup preserves the denial boundary without claiming to implement authentication.
- Bazaar, private journal and Atlas compositions describe plan contracts where no corresponding enabled Laravel feature establishes behavior. They do not invent a game transport or promise working endpoints. No Game repository content or fixtures were required or copied.

No real character, content coordinates, world version, snapshot hash, account identifier or credentials appear. Mira and Brzask are invented labels. The Atlas is explicitly schematic, not an asserted map; the statistics table is an accessible chart alternative with invented samples, not telemetry. Runtime page owners must use Fluent translations, their existing guards and off-by-default flags before implementation. Unknown or ambiguous authority denies access/action.

## Reproduction and validation

`copy.json` is the paired design-only i18n catalog, following T39's static-gallery precedent. `python3 docs/design/t39b/build.py` escapes translations and generates twelve HTML artifacts. This is build-time tooling only; no runtime code or second application i18n layer is added.

Install `playwright-core` and `@axe-core/playwright` under `/tmp/portal-t39b-browser`, start the repository-root HTTP server, then run `node docs/design/t39b/check.cjs`. The check captures all twelve documents at 360 and 1280 px, checks overflow, skip-link activation and native keyboard disclosures without artifact JavaScript, disabled operations and field-error associations, and scans the expanded state examples with axe WCAG A/AA tags through 2.2. T39 supplies the tested contrast pairs and 44 px control targets. Automated scans do not certify complete WCAG conformance; reading order, wording and final interactive maps/charts still need human review during implementation.

Owner approval is outstanding. Do not treat these artifacts or passing checks as approval to implement the pages or enable their features.
