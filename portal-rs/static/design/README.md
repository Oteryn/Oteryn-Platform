# Oteryn interface foundations

T39 provides a new visual foundation: ink surfaces, warm amber wayfinding, restrained borders and a system sans-serif stack. It does not copy the Laravel portal. These are component samples, not screen mockups or enabled game/money features. Screen composition and owner approval belong to T39b.

Open `/static/design/index.html` through the portal's existing static-file service, or serve `portal-rs/static/` locally. Every component appears in dark (English) and light (Polish), simultaneously. The surrounding gallery follows `prefers-color-scheme`. Explicit `data-theme="dark"` and `data-theme="light"` scopes override the preference. No font, icon, script or stylesheet uses a CDN.

## Integration contract for T01 and page owners

Load `/static/css/tokens.css`, then `/static/css/components.css`. Put `class="op-theme"` on the layout body. Include `components/icons.html` once per document. Import the individual Askama macro files; `components/gallery.html` is a compile-tested usage fixture for all signatures, not a production layout. `gallery.css` belongs only to the gallery. Keep every color, spacing, radius, shadow and type choice in tokens; do not set text opacity or place text over decorative shadows/backdrops.

All product-facing labels, hints, errors, captions, navigation names and button text are macro arguments. Pass Fluent-translated Polish or English strings from the caller's i18n layer. Components never choose a language and contain no hardcoded user-facing copy. The static gallery is generated from `locales/en.tsv` and `locales/pl.tsv`; these catalogues are for the static design artifact only and do not add a second product translation service. TSV rows are `key<TAB>translation`, UTF-8. Keep both key sets identical.

IDs must be unique, trusted component IDs; URLs must come from trusted application routes (HTML escaping does not validate URL schemes). Pass rectangular table rows matching the header count, and side-panel values matching the label count. Dynamic content remains autoescaped; do not pass safe/raw HTML. Button macros use `type="button"`; page owners provide submission, CSRF and server validation in their own forms. Inline field errors use `aria-describedby` and `aria-invalid`. Dynamic alerts should be inserted only when there is a new message, avoiding repeated live announcements.

Tabs are ordinary server-navigation links with `aria-current="page"`, not a scripted ARIA tablist. Disabled pagination uses non-interactive spans. Add a descriptive current-page string, such as “Page 1 of 2”. Frame macros are explicitly labeled placeholders; map/chart integrations must supply text alternatives and chart data tables when added. No game or money action is implemented here, so no feature flag is added.

For modal/drawer enhancement, load `/static/design/drawer.js` with `defer`. Native `<details>` provides a readable keyboard-accessible fallback without JavaScript. Enhancement creates `<dialog>` elements and labeled triggers: native modal focus trapping, Escape dismissal, background inertness, a visible close button, and focus restored to the trigger. The dialog inherits its containing theme. No inline script/style or external dependency is needed under the portal CSP. The script is below 5 KB and handles only overlays.

## Accessibility and responsive behavior

Normal text uses the stricter 4.5:1 threshold at every size. Focus rings and control borders meet 3:1. Status pairs carry meaning through words/icons as well as color. Links are underlined, controls are at least 44 CSS px tall, fields have explicit labels, focus is visible, and reduced-motion disables the loading animation. At 360 px, content wraps in one column; only the labeled, keyboard-focusable table region scrolls horizontally. Do not disable zoom. Use landmarks, headings and unique accessible names when composing pages. A component library cannot certify accessibility of future pages.

The contrast manifest is embedded in `tokens.css` as `@contrast` comments. Ratios below use the WCAG sRGB transfer function and relative luminance, rounded only for display; tests compare unrounded values. Text, muted text, links, errors, border and focus colors are verified against all three neutral surfaces. Status text uses only its paired status background; on-accent text uses only accent. Shadow/backdrop colors carry no text. Disabled buttons remain fully legible.

| Foreground / background | Dark | Light | Minimum |
| --- | ---: | ---: | ---: |
| text / canvas | 16.20 | 14.62 | 4.5 |
| text / surface | 14.48 | 16.24 | 4.5 |
| text / raised | 12.26 | 13.32 | 4.5 |
| muted / canvas | 9.75 | 6.42 | 4.5 |
| muted / surface | 8.71 | 7.13 | 4.5 |
| muted / raised | 7.38 | 5.85 | 4.5 |
| accent / canvas | 10.67 | 6.39 | 4.5 |
| accent / surface | 9.53 | 7.09 | 4.5 |
| accent / raised | 8.07 | 5.82 | 4.5 |
| danger / canvas | 10.59 | 6.83 | 4.5 |
| danger / surface | 9.46 | 7.58 | 4.5 |
| danger / raised | 8.01 | 6.22 | 4.5 |
| on-accent / accent | 10.67 | 7.09 | 4.5 |
| success / success-bg | 8.97 | 7.09 | 4.5 |
| warning / warning-bg | 9.10 | 7.36 | 4.5 |
| danger / danger-bg | 8.63 | 6.50 | 4.5 |
| info / info-bg | 8.76 | 6.94 | 4.5 |
| focus / canvas | 10.67 | 6.39 | 3.0 |
| focus / surface | 9.53 | 7.09 | 3.0 |
| focus / raised | 8.07 | 5.82 | 3.0 |
| border / canvas | 5.65 | 4.40 | 3.0 |
| border / surface | 5.05 | 4.88 | 3.0 |
| border / raised | 4.27 | 4.01 | 3.0 |

## Icons and licensing

`templates/components/icons.html` is an inline SVG sprite adapted from [Lucide 0.468.0](https://github.com/lucide-icons/lucide/tree/0.468.0), itself derived from Feather. Icons used/adapted: arrow-right, circle-check, info, triangle-alert, map, chart-no-axes-combined, inbox and loader. Stroke geometry is simplified for a shared 24×24 grid; icons inherit `currentColor`. Lucide is ISC-licensed and Feather portions are MIT-licensed. The full upstream notice is preserved in [ICON-LICENSE](ICON-LICENSE). Decorative uses have `aria-hidden="true"`; an icon-only control must receive its own translated accessible name. Typography uses platform system fonts, with no downloaded font files.

## Checks and regeneration

From `portal-rs/`, with the pinned toolchain on PATH:

```sh
python3 static/design/check.py
# After changing partials or gallery translations:
python3 static/design/check.py --update
rustfmt --edition 2024 --check static/design/tests.rs
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

The runner builds the locked workspace's `oteryn-web`, locates its exact Askama artifact in Cargo's JSON output, and compiles `tests.rs` in a temporary directory against that artifact and the existing Askama template configuration. This keeps Rust tests and compile fixtures within T39's allowed paths without changing the workspace, router, manifests or lockfile. It runs five Rust tests plus strict lint checks: every declared color pair in both themes, refusal of insufficient contrast (including the 4.48 boundary), malformed/missing input, compilation/rendering of every partial with exact gallery parity and bilingual key parity, and escaped invalid input with accessible refusal/error states. These additional checks must be run explicitly; the workspace's `cargo test` does not discover tests under static assets. There are no database integration tests in this presentation-only task (0 DB tests); no SQL or database dependency was added.

## Browser verification

Install test-only dependencies outside the repository, serve the static directory, and run:

```sh
npm install --prefix /tmp/portal-t39-browser --cache /tmp/portal-t39-npm-cache --no-audit --no-fund playwright-core @axe-core/playwright
python3 -m http.server 8099 --bind 127.0.0.1 --directory portal-rs/static
# In another terminal, from the repository root:
node portal-rs/static/design/browser-check.cjs
```

The script defaults to `/usr/bin/chromium`; override `CHROMIUM_PATH`, `DESIGN_BROWSER_MODULES` or `DESIGN_GALLERY_URL` for another local installation/server. It checks both themes at 360 and 1280 px, no page overflow, keyboard entry/Tab containment/Escape and close-button dismissal/focus restoration for all overlays, keyboard table scrolling, invalid-field descriptions, disabled controls, reduced motion, native disclosures without JavaScript, and both system color preferences. It runs axe WCAG 2 A/AA, 2.1 AA and 2.2 AA checks on the gallery and each open overlay. Screenshots go to `/tmp/`, not the repository. A passing automated scan does not replace page-level accessibility review.

T39 local result: five Rust design tests passed (23 pairs × two themes), strict design lint passed, 360/1280 px browser checks passed with zero axe violations, JavaScript-free disclosures passed, and system preference/explicit theme checks passed. The drawer script is 1,807 bytes. Workspace acceptance: fmt passed, Clippy passed with warnings denied, and eight unit tests passed with no failures; doc tests passed. `PORTAL_TEST_DATABASE_URL` was set for acceptance; zero DB integration tests exist in the T39/current-main suite. The requested environment bootstrap was attempted and stopped at the unavailable Docker socket; all presentation and workspace checks ran using the pinned native Rust 1.97.0 installation.
