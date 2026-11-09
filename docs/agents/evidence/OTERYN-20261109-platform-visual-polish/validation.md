# Literal reference template — current validation

**Date:** 2026-11-09 UTC. **Issue/PR:** #1798 / #1799. **Admission main:** `dbd91f0325ff0b8a3c0512b7fb161db1b305f77d`. **Verified material predecessor:** `1e49df4f2f701eafb70322713f249e384f3fe19d`. The next API-native successor contains the complete frozen delta; its returned SHA and CI belong to the live PR. This evidence does not claim deployment or complete bitmap identity.

## Source and implementation

The owner explicitly required literal slicing. Source PNG: **1672×941**, SHA256 `08090697dd979efca7fa0afd2b4e97751af1b4723245a9aca904df7cd277725a`. Product assets are **85 WebP files, 2,737,856 bytes**, with coordinates, source hash, exact masks, encoder versions and per-file hashes in `portal-rs/static/img/reference/layout.json`. `portal-rs/tools/extract_home_reference.py --source ORIGINAL.png` checks the source identity and uses that selected manifest. The source PNG itself is not deployed as a live-data page.

An isolated reproduction using the final extractor produced all **85 assets and layout.json byte-for-byte identically**. The extraction checks every retained RGBA pixel against the original and every cleared pixel against transparent zero RGBA; the decoded lossless WebP must equal the expected crop. Independent asset review confirmed the selected files and masks. Scratch records: `reference-reproduction-final/evidence.json`, `reference-slices/tight-mask-verification.json`, and `oteryn-reference-final-evidence.json`.

Static Polish artwork/labels are slices with matching accessible names. Native status/news/ranking remain application values. Server-save/experience/loot fields have no backing HomeStatus fields and display `—`; sample values are removed from deployed surfaces. Empty lists do not render sample cards or players. Original row artwork is attached only to real rendered records. Medals use actual rank, preserving gaps caused by privacy filtering. Sparse registries report each unavailable group explicitly.

## Frozen browser and Rust results

- **Native Askama + Chromium: PASS, 96 cases / 10,422 checks / 0 findings.** Run 19:57:52–20:00:03 UTC, widths 320/390/681/720/900/1280/1440/1672, height 941, PL/EN, observed/unavailable/populated/signed-in/404 states, keyboard and observed-state touch. All native snapshots were fresh; source hashes and all 104 served resources remained unchanged. Source: `portal-reference-browser-check-final-fresh.json`.
- Checks cover accessible headings, landmarks, eight navigation slots, all 39 captured registry links, menu opening/closing and reachability, CTA/news/ranking hit testing, images, overflow, account guards, POST/CSRF markup and honest data states. No form submission or destination mutation was performed.
- **Fresh focused Rust tests: PASS, 53 reported passed / 1 ignored, exit0.** Command: `cargo +1.97.0 test --offline --locked -p oteryn-ui -p oteryn-web`; UI 10, web 27, design-system 5, ratelimit 11. Source: `portal-reference-tests-final.log`. **PORTAL_TEST_DATABASE_URL was unset:** conditional database tests returned without database assertions; their runner labels do not establish PostgreSQL coverage.
- **Focused Clippy, formatting and whitespace: PASS.** `cargo +1.97.0 clippy --offline --locked -p oteryn-ui -p oteryn-web --all-targets -- -D warnings`, `cargo +1.97.0 fmt --all --check`, `git diff --check`. Source: `portal-reference-clippy-final.log`; Clippy finished in 11.89s. Full-workspace/DB qualification belongs to CI.
- **Independent source/asset review: PASS, no blocking regression found.** Includes conditional source surfaces, real ranks, sparse registry, escaping, logout, layer hit testing and manifest/extractor consistency.

The first local Rust log reused an artifact whose `.d` pointed to another workspace and is explicitly superseded. UI/web and both native renderer targets were selectively cleaned/rebuilt. Their current dependency records point to the current checkout. The fresh build also exposed two established exact shell-markup assertions; the implementation preserved `hx-head`/`hx-foot` and moved current-page state to the wrapper without weakening tests.

## Pixel comparison — exact scope and remaining difference

`portal-reference-check.cjs` captured the real native fixture at **1672×941, DPR1**, with no CSS/DOM overrides, screenshot resizing, alignment or comparison masks. Document and scroll geometry are **1672×941**.

| Measurement | Result |
| --- | --- |
| Full-frame strict bitmap identity | **FAIL: 97,883 / 1,573,352 pixels differ (6.2213%)** |
| Full-frame mean RGB delta | 1.963374587 on the 0–255 channel scale |
| Full-frame identical pixels | 1,475,469 (93.7787%) |
| Header, five feature cards, gallery, footer regions | Zero differing pixels in each region |
| Additional location analysis outside the three dynamic-data panels | 1,192,986 pixels, zero differences |

All full-frame differences lie inside the status/news/ranking panels. Native typography, category labels, real/unavailable status values and the bounded backgrounds cleared behind editable data differ from the flattened image. The original editable font family is unknown. **The complete image result remains FAIL and is not converted into PASS by the narrower static-region analysis.** No panel/frame mask was passed to the comparator.

Sources: `reference-pixel-final/report.json` and `static-layout-verification.json`; actual screenshot SHA256 `82d3cafb9367cf69f2ee6d9fbcc998a4a0170c49d749acef79a8e5ee3dd1e983`. Mobile 390 screenshots show the source navigation edge restored and the tiny forest texture no longer enlarged; no missing images, browser errors or horizontal overflow.

## Reviewable artifact

`oteryn-podglad.html` is a self-contained comparison with **Wzorzec / Szablon / Odczytany stan strony**, fit/1:1 controls and native 1672/390 widths. The source-fixture label is outside the product iframe. The observed-state view is explicitly a captured render without a live production database connection. Native bodies remain unchanged apart from resource/link transport for offline viewing.

Offline validation passed for both native views and the original PNG, keyboard/scale controls, zero external requests, zero browser errors and zero missing images. Artifact SHA256 `de22ea7782e5b59c3425cc5692c946d43a52256f9dbdae8cce5b27d0b2622efe`, 9,841,738 bytes. Sources: `oteryn-reference-review-check.json`, `oteryn-reference-final-evidence.json`.

## Frozen source SHA256

| File relative to repository root | SHA256 |
| --- | --- |
| `portal-rs/crates/ui/src/lib.rs` | `ef2534ad4c0015df23a135f9a858357bf099fb74ce98a43251c32eb014aaf43f` |
| `portal-rs/crates/ui/i18n/pl.ftl` | `414ffc016709aefd063594bab9825f2fd02fb6c69d96def302311d830f5d50e9` |
| `portal-rs/crates/ui/i18n/en.ftl` | `c00fd3b3dbcb02cdc763f775931af222de7f54d6a308733d853ec6a675d1ff4c` |
| `portal-rs/templates/pages/home.html` | `5bcfdaf575027c9aaad4c6ba3b2748b68372bd16cc2cc3a34d13fdd05806d5fc` |
| `portal-rs/templates/layout.html` | `93e99f65d4cdb84020e15737f3a312b8cf3ca68b2935935f3836653100483d13` |
| `portal-rs/templates/components/shell-header.html` | `5d217ece837e5a052a9d2ba96f21962454bb635b05a9f46c29986cafde41b581` |
| `portal-rs/templates/components/shell-footer.html` | `98aefeb6e1949ed6f465ead641b4f5551adfa22e9ee419772320dd4c0ba19b08` |
| `portal-rs/static/css/home.css` | `20a4628ff798d20c9257b85d169caeded1babf30e695d29e9b0facbdc6a84e1e` |

Extractor SHA256: `272381cb7f003637bb1dde621dcbf89550c034c3b7be1dd20566b9aa6b66a876`. Per-asset hashes and geometry are committed in the selected layout manifest.

## Limits and next gate

This is native-renderer/browser verification, not backend or live-production E2E. No database/authenticated-session mutation or client download was exercised. The download link preserves the existing mounted `server-information#download` fallback; current CMS anchor content and final client download remain unverified. Browser coverage is Chromium with keyboard/touch emulation, not a cross-browser or comprehensive accessibility audit.

Observe **Portal RS CI, CI/platform-gate, Agent Governance, CodeQL and the existing path-routed heavy workflows** on the actual new successor SHA. Previous candidate checks are superseded. Root remains the sole publishing writer; one complete tree/sole-parent material commit/non-force expected-head update and immediate readback are required. No production effect or deployment is claimed.

---

# Historical evidence — superseded first interpretation

The following retained record belongs only to the earlier 98d0773/1e49df4 candidate and cannot establish acceptance of the literal reconstruction above.

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
