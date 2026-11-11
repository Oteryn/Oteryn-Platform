---
task_id: OTERYN-20261109-platform-visual-polish
governing_issue: 1798
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - portal-rs/templates/pages/home.html
  - portal-rs/static/css/home.css
  - portal-rs/crates/ui/src/lib.rs
optional_reads: []
---

# OTERYN-20261109-platform-visual-polish

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1798. Draft PR: https://github.com/Oteryn/Oteryn-Platform/pull/1799.

Build the existing Rust portal from literal, lossless slices of the owner's 1672×941 reference. Preserve its original architecture, fantasy art, ornaments, buttons, Polish static lettering and native-size composition. Keep status, news and ranking as truthful application data; retain mounted routes, account/CSRF behavior and accessible PL/EN responsive rendering. The owner's correction supersedes the earlier visual interpretation.

Admission main is `dbd91f0325ff0b8a3c0512b7fb161db1b305f77d`. Root is the sole publisher on `chatgpt/platform-visual-polish-20261109`. The complete new material delta is prepared against verified predecessor `1e49df4f2f701eafb70322713f249e384f3fe19d`; one API-native successor will carry the code, selected assets and this evidence together. No deployment is authorized or claimed by this packet.

## Acceptance criteria

- [x] Extract the original artwork without resampling, generating new art or retaining sample live values.
- [x] Match the native 1672×941 geometry and verify source graphics outside the three dynamic panels pixel-for-pixel.
- [x] Preserve real mounted routes, data/empty/unavailable states, escaped output and POST/CSRF logout.
- [x] Verify PL/EN, keyboard/touch, signed-in state, sparse navigation, non-contiguous ranks and responsive layout.
- [x] Deliver a native-body comparison preview with clearly labelled source-fixture and captured-state views.
- [ ] Qualify the complete material successor with exact-SHA GitHub workflows.
- [ ] Complete owner visual review of the remaining native text/data differences; full bitmap identity has not been established.

## Ownership

```yaml
owned_paths:
  - portal-rs/static/css/home.css
  - portal-rs/templates/layout.html
  - portal-rs/templates/components/shell-header.html
  - portal-rs/templates/components/shell-footer.html
  - portal-rs/templates/pages/home.html
  - portal-rs/static/img/reference/**
  - portal-rs/tools/extract_home_reference.py
  - portal-rs/crates/ui/i18n/pl.ftl
  - portal-rs/crates/ui/i18n/en.ftl
  - portal-rs/crates/ui/src/lib.rs
  - docs/agents/tasks/active/OTERYN-20261109-platform-visual-polish.md
  - docs/agents/tasks/active/OTERYN-20261108-portal-rs-home-template.md
  - docs/agents/evidence/OTERYN-20261109-platform-visual-polish/**
modules:
  - shared-portal-shell
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

Ownership of the predecessor packet is limited to its already-recorded `archive_pending` reconciliation for merged PR1782. Historical evidence remains preserved.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: '2026-11-09T20:04:37Z'
head: 1e49df4f2f701eafb70322713f249e384f3fe19d
branch: chatgpt/platform-visual-polish-20261109
pr: 1799
status: validating
context_routes:
  - web-cms
  - testing
  - agent-governance
owned_paths:
  - portal-rs/static/css/home.css
  - portal-rs/templates/layout.html
  - portal-rs/templates/components/shell-header.html
  - portal-rs/templates/components/shell-footer.html
  - portal-rs/templates/pages/home.html
  - portal-rs/static/img/reference/**
  - portal-rs/tools/extract_home_reference.py
  - portal-rs/crates/ui/i18n/pl.ftl
  - portal-rs/crates/ui/i18n/en.ftl
  - portal-rs/crates/ui/src/lib.rs
  - docs/agents/tasks/active/OTERYN-20261109-platform-visual-polish.md
  - docs/agents/tasks/active/OTERYN-20261108-portal-rs-home-template.md
  - docs/agents/evidence/OTERYN-20261109-platform-visual-polish/**
proven:
  - The owner requires a literal template sliced from the supplied 1672x941 image; its SHA256 is 08090697dd979efca7fa0afd2b4e97751af1b4723245a9aca904df7cd277725a.
  - The final 85 lossless WebP assets and layout.json reproduce byte-for-byte through the committed extractor; retained source pixels and removal masks were independently verified.
  - Fresh native Askama browser validation passed 96 cases and 10422 checks with zero findings at eight widths in PL and EN; sources and all 104 served resources remained unchanged.
  - The native 1672x941 render has matching document geometry and zero differences across all 1192986 pixels outside the three dynamic-data panels. The complete unmasked image still differs in 97883 pixels (6.2213 percent); full bitmap identity is not claimed.
  - Fresh focused Rust tests passed 53 with one ignored test; focused Clippy, formatting, diff check and independent code/asset review passed. Conditional database tests did not exercise PostgreSQL locally.
  - The three-view offline comparison preserves the native bodies, uses no external requests and passed desktop/mobile/keyboard checks.
  - The task branch remains at predecessor 1e49df4 before selecting one API-native material successor; main remains at admission dbd91f0. Draft PR1799 and Issue1798 are open.
derived: []
unknown:
  - CI outcomes for the forthcoming complete material successor; final SHA and check evidence belong to the native PR records.
  - The editable source/font family behind dynamic lettering in the flattened reference; native text and current data prevent complete bitmap identity.
  - The current CMS download fragment and final client download result; only the existing mounted server-information fallback is verified.
  - Jira mapping/synchronization remains unknown.
conflicts: []
first_failure:
  marker: resolved
  evidence: Mobile footer menus previously escaped the viewport and were corrected. Fresh compilation exposed exact shell-class integration assertions, resolved by preserving the established header/footer classes without weakening tests.
rejected_hypotheses:
  - The first interpretation is superseded and does not satisfy the owner correction.
  - The first local Rust log with artifacts from a different workspace is not evidence for current sources; UI/web and both renderer targets were selectively rebuilt.
changed_paths:
  - portal-rs/static/css/home.css
  - portal-rs/templates/layout.html
  - portal-rs/templates/components/shell-header.html
  - portal-rs/templates/components/shell-footer.html
  - portal-rs/templates/pages/home.html
  - portal-rs/static/img/reference/**
  - portal-rs/tools/extract_home_reference.py
  - portal-rs/crates/ui/i18n/pl.ftl
  - portal-rs/crates/ui/i18n/en.ftl
  - portal-rs/crates/ui/src/lib.rs
  - docs/agents/tasks/active/OTERYN-20261109-platform-visual-polish.md
  - docs/agents/tasks/active/OTERYN-20261108-portal-rs-home-template.md
  - docs/agents/evidence/OTERYN-20261109-platform-visual-polish/**
validation:
  - command: Exact-source extraction and isolated selected-manifest reproduction
    result: PASS
    evidence: 85 assets, 2737856 bytes; all 85 outputs and layout.json byte-identical. Durable details in validation.md.
  - command: Fresh native Askama and Chromium matrix
    result: PASS
    evidence: portal-reference-browser-check-final-fresh.json; 96 cases, 10422 checks, 0 findings; 2026-11-09 19:57:52–20:00:03 UTC; source/resource hashes unchanged.
  - command: Unmasked 1672x941 full-frame pixel comparison
    result: FAIL
    evidence: 97883/1573352 pixels differ (6.2213 percent), all inside status/news/ranking panels. Header, features, gallery and footer have zero differences. Geometry passes; the strict complete-bitmap criterion is not relabelled PASS.
  - command: cargo +1.97.0 test --offline --locked -p oteryn-ui -p oteryn-web
    result: PASS
    evidence: portal-reference-tests-final.log after selective clean/rebuild; 53 reported passed, 1 ignored, exit0. PORTAL_TEST_DATABASE_URL unset; no local DB integration claim.
  - command: Focused Clippy, cargo fmt --all --check, git diff --check
    result: PASS
    evidence: portal-reference-clippy-final.log; focused UI/web --all-targets -D warnings, exit0; formatting and whitespace checks passed.
  - command: Native comparison export and offline browser check
    result: PASS
    evidence: oteryn-reference-final-evidence.json; 1672/390 views, original reference, keyboard/scale controls, zero missing images, browser errors or external requests; native body retained.
  - command: Final candidate GitHub workflows
    result: NOT_RUN
    evidence: Observe Portal RS CI, CI/platform-gate, Agent Governance, CodeQL and existing path-routed workflows on the API-native material successor. Earlier candidate results do not qualify it.
blockers:
  - none
next_action: Observe the exact-SHA repository checks on the published material successor in draft PR1799, then complete owner visual review.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: The source branch remains intentionally active for draft PR1799, exact-SHA CI and owner visual review.
source_branch_evidence: https://github.com/Oteryn/Oteryn-Platform/pull/1799
```

## Notes

The checkpoint head records the last observed predecessor; it cannot self-reference its containing successor. Final publication identity and CI results must be read from the live PR, not inferred from this packet. Production deployment requires its separate task-specific authority under the unchanged repository bootstrap.

The strict full-frame comparator remains FAIL because the functional template renders native text and real/missing values. Its masks list is empty, and no ornamental region or panel was excluded to turn that result into PASS. The separate zero-difference static-region measurement and extraction proof are narrower facts.

## Superseded history

The initial interpretation was published as `98d0773863681c11f8c1b8f6a225121f0f109745`; `1e49df4f2f701eafb70322713f249e384f3fe19d` recorded PR1799 identity. Earlier 96-case/2912-check and 51-test evidence applies only to those earlier sources. It is retained under the historical section of validation.md and does not qualify this literal reconstruction. No version of this task has been deployed.
