# Portal visual polish — validation evidence

**Date:** 2026-11-09 (UTC). **Issue:** Oteryn/Oteryn-Platform#1798. **Base HEAD:** `dbd91f0325ff0b8a3c0512b7fb161db1b305f77d`. This record identifies the six changed files in material candidate `98d0773863681c11f8c1b8f6a225121f0f109745`, published in PR #1799. The subsequent task-identity/evidence commit leaves these product bytes unchanged. It is not a claim about final CI or deployment.

## Verified scope and results

- **Native renderer browser QA on frozen sources: PASS — 96 cases, 2,912 checks, 0 findings**, 18:49:17–18:50:16 UTC. PL/EN at widths **320, 390, 681, 720, 900, 1280, 1440, 1672**, height 900. For each language/width: observed anonymous state, all data unavailable, explicit populated fixture, signed-in fixture, and shared-shell 404 with keyboard checks; observed state additionally tested with touch. Source: `portal-browser-check-final-results.json`. This final run follows the background CSS edits and restoration of baseline `shell-welcome`.
- Checked horizontal overflow, loaded visible images, full hero illustration containment, a single visible H1, shared landmarks/skip target, navigation overlap and viewport bounds, native dropdown opening/closing with focus retained, keyboard entry and reachability of first/last menu links, localized missing/empty/populated states, anonymous/signed-in link differences, and logout **POST + CSRF markup**. No logout request or other form submission was made.
- **Focused Cargo test command exited 0:** `cargo +1.97.0 test --offline --locked -p oteryn-ui -p oteryn-web`. Runner totals: **51 reported passed, 0 failed, 1 ignored** (UI 8, web 27, design-system 5, ratelimit 11). Source: `portal-focused-tests-frozen.log`, including the command, DB disposition and exit code. **Database integration was not exercised:** `PORTAL_TEST_DATABASE_URL` was unset; DB-dependent tests conditionally return without executing their database assertions. Their runner `ok` labels are not database coverage. The ignored test is the separate PostgreSQL search/browser test.
- **Focused Clippy completed successfully:** `cargo +1.97.0 clippy --offline --locked -p oteryn-ui -p oteryn-web --all-targets -- -D warnings`; source `portal-clippy-focused.log`. `git diff --check` passed; root also verified final Cargo formatting. Full workspace Clippy was blocked by uncached dependencies/network 403 and remains for CI; no full-workspace lint pass is claimed.
- **Final visual inspection: PASS** at desktop 1672 px and mobile 390 px. The hero joins are softened, the repeated side-frame artifact is removed, and all five gallery images remain visible on mobile. This is a direct screenshot review, separate from the interaction matrix.
- **Standalone observed PL export passed file-URL Chromium checks at 1440/390:** all 25 images and required fonts loaded, no overflow, external requests or page errors. Three stylesheets/assets/fonts are embedded; preloads removed; `#main` retained; page links point to `https://oteryn.com/`. No fixture/authenticated data, forms, scripts or new UI were added. Source: `oteryn-podglad-check.json`.

## Method and limits

The scratch harness calls the real compiled Askama `oteryn_ui::home`/`error` renderer and verifies its response status before saving documents. Chromium serves those documents with the repository CSS/assets. This is **native-renderer and browser layout validation, not backend E2E**: no live database, authenticated session, API integration, actual destination navigation, or production mutation. The observed state is the captured combination of unavailable status, successfully empty news and unavailable rankings. Populated counts, players and news are explicitly marked synthetic fixtures. Browser coverage is Chromium with keyboard/touch emulation; it is not a cross-browser or comprehensive accessibility audit. The static host's HTTP status is not backend route validation.

## Frozen source SHA256

Paths below are relative to the repository root.

| Changed file | SHA256 |
| --- | --- |
| `portal-rs/crates/ui/i18n/en.ftl` | `0f78984ab0f025a593a5875265726b237e6ee9a0b094ff3f3219f0ee0e967d14` |
| `portal-rs/crates/ui/i18n/pl.ftl` | `4c4a21fecabf47be6dac9f9fb06abd81daa6472f1be7479df3c213a2dae2e9cf` |
| `portal-rs/crates/ui/src/lib.rs` | `cccb124d8f9589530871339007952cedd20647062ac4061d10d3cd340bcfba1e` |
| `portal-rs/static/css/home.css` | `a5306a6e566eb23b0d159e0f45a90bc6267228c5bc9fc3bdbbc913e88a39b17c` |
| `portal-rs/templates/layout.html` | `93e99f65d4cdb84020e15737f3a312b8cf3ca68b2935935f3836653100483d13` |
| `portal-rs/templates/pages/home.html` | `2ed43208398acd1631c8ad4975dd10aaf9d296b58661c417837e0c1cb517551e` |

Evidence logs and large JSON reports remain outside the repository. Full-matrix rerun: `node /workspace/scratch/0d29a6985a5d/portal-browser-check.cjs --output /workspace/scratch/0d29a6985a5d/portal-browser-check-final-results.json`. Native observed PL document was regenerated at **18:48:59 UTC**, after the frozen sources; SHA256 `02abea7237a002f71b5b5b67b17d0f80d5238adfbc5064d11cb9c805a0823562`.
