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

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1798.

Improve the existing Rust portal at `/pl/` and `/en/`: refine the brand header, compose the supplied artwork without narrow character crops, improve typography and spacing, and keep navigation, status, news and rankings usable across desktop and mobile widths. Work remains within the existing Rust/Askama/Fluent presentation layer and uses the retained owner-supplied assets, including the logo already embedded in `sky.webp`.

Admission main: `dbd91f0325ff0b8a3c0512b7fb161db1b305f77d`. The canonical task branch is `chatgpt/platform-visual-polish-20261109`. The root coordinator is the sole publisher. The material candidate is `98d0773863681c11f8c1b8f6a225121f0f109745`. Draft PR https://github.com/Oteryn/Oteryn-Platform/pull/1799 records the complete change. This required lifecycle/evidence successor records the returned PR identity; it does not change the product files.

## Acceptance criteria

- [x] Keep the existing Rust presentation layer and mounted-route navigation.
- [x] Keep unavailable and empty data states truthful; sample players, counts and news exist only in the explicitly labelled external QA fixture.
- [x] Verify PL/EN home, signed-in chrome, unavailable states, populated fixtures and shared 404 layout across the recorded responsive widths.
- [x] Preserve semantic landmarks, native disclosure controls, escaped output and the existing POST/CSRF logout behavior.
- [x] Complete the final desktop and mobile visual review of the frozen product changes.
- [ ] Publish the complete bounded change, record the real draft PR number and observe the repository checks on the final candidate SHA.

## Ownership

```yaml
owned_paths:
  - portal-rs/static/css/home.css
  - portal-rs/templates/layout.html
  - portal-rs/templates/pages/home.html
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

Ownership of the predecessor packet is limited to reconciling its verified terminal PR #1782 through the existing `archive_pending` transition. Its historical implementation and validation evidence are preserved.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-09T18:55:14Z
head: 98d0773863681c11f8c1b8f6a225121f0f109745
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
  - portal-rs/templates/pages/home.html
  - portal-rs/crates/ui/i18n/pl.ftl
  - portal-rs/crates/ui/i18n/en.ftl
  - portal-rs/crates/ui/src/lib.rs
  - docs/agents/tasks/active/OTERYN-20261109-platform-visual-polish.md
  - docs/agents/tasks/active/OTERYN-20261108-portal-rs-home-template.md
  - docs/agents/evidence/OTERYN-20261109-platform-visual-polish/**
proven:
  - Native Askama rendering uses the real oteryn-ui home, chrome and error functions, without a database or live session mutation.
  - The final browser matrix passed 96 cases and 2912 checks, with zero failed cases or findings, across PL and EN at widths 320, 390, 681, 720, 900, 1280, 1440 and 1672 pixels; it finished at 2026-11-09T18:50:16.348Z.
  - The focused Rust log reports 51 passed tests and 1 ignored browser-search test; database-conditional tests do not establish PostgreSQL coverage when their database variable is unset.
  - The focused Rust lint completed successfully and the root coordinator recorded a passing final desktop and mobile visual review.
  - The existing predecessor PR 1782 is merged at 0571e5b73b3b30690861b38ba7bf5b39b215117f and cannot retain active implementation ownership.
derived: []
unknown:
  - Exact-candidate CI results for the final PR 1799 lifecycle successor remain pending.
conflicts: []
first_failure:
  marker: none
  evidence: The earlier banner overflow was corrected before the recorded passing browser matrix.
rejected_hypotheses: []
changed_paths:
  - portal-rs/static/css/home.css
  - portal-rs/templates/layout.html
  - portal-rs/templates/pages/home.html
  - portal-rs/crates/ui/i18n/pl.ftl
  - portal-rs/crates/ui/i18n/en.ftl
  - portal-rs/crates/ui/src/lib.rs
  - docs/agents/tasks/active/OTERYN-20261109-platform-visual-polish.md
  - docs/agents/tasks/active/OTERYN-20261108-portal-rs-home-template.md
  - docs/agents/evidence/OTERYN-20261109-platform-visual-polish/validation.md
validation:
  - command: Focused oteryn-ui and oteryn-web Rust test run
    result: PASS
    evidence: portal-focused-tests-frozen.log reports UI 8, web 27, design-system 5 and rate-limit 11 passed; 1 ignored test; PORTAL_TEST_DATABASE_URL was unset and conditional database skips remain a verification limit. The compact durable summary is docs/agents/evidence/OTERYN-20261109-platform-visual-polish/validation.md.
  - command: Focused oteryn-ui and oteryn-web Rust lint
    result: PASS
    evidence: portal-clippy-focused.log ends with the completed dev profile. Full workspace lint could not obtain an uncached dependency through the environment network and is not claimed as passing; the durable validation summary records this limit.
  - command: Native Askama browser matrix in portal-browser-check.cjs
    result: PASS
    evidence: portal-browser-check-final-results.json records 96 cases, 2912 checks and zero findings; keyboard and touch, PL/EN, observed/unavailable/signed-in/populated/404 states. Results are summarized in docs/agents/evidence/OTERYN-20261109-platform-visual-polish/validation.md.
  - command: git diff --check
    result: PASS
    evidence: Completed successfully during the final diff review before candidate selection.
  - command: Final visual review after cosmetic CSS adjustment
    result: PASS
    evidence: The root coordinator completed final desktop and mobile visual inspection after the cosmetic changes and confirmed the result; the product delta is frozen.
  - command: Final candidate GitHub checks
    result: NOT_RUN
    evidence: PR 1799 is open in draft with the verified material predecessor recorded above. Observe Portal RS CI, CI/platform-gate, Agent Governance and existing path-routed workflows on the final lifecycle successor; no final CI pass is claimed here.
blockers:
  - none
next_action: Observe native GitHub checks on the final PR 1799 lifecycle successor and resolve concrete failures before readiness.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: The visual-polish task is active in draft PR 1799 while final CI and review are pending.
source_branch_evidence: https://github.com/Oteryn/Oteryn-Platform/pull/1799; the branch remains intentionally active until verified integration or terminal closeout.
```

## Notes

Checkpoint `head` records the last observed material candidate `98d0773863681c11f8c1b8f6a225121f0f109745`. The lifecycle-only successor records the actual PR identity and keeps all six material source files byte-identical. This checkpoint does not claim that the successor itself has passed CI. Final exact-head CI cannot be replaced by the local rendered-fixture evidence; its results belong to the native PR/check records.

The report names above identify the retained scratch verification outputs. The compact durable evidence is `docs/agents/evidence/OTERYN-20261109-platform-visual-polish/validation.md`; it records outcomes, commands and limitations without committing screenshots or full JSON reports. Do not copy fixture values into product data.

This change does not alter authentication, production data, server runtime, database schemas or hosting configuration. Deployment is not claimed by this checkpoint. Jira mapping/synchronization is unknown.
