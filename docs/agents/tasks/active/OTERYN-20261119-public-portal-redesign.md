---
task_id: OTERYN-20261119-public-portal-redesign
governing_issue: 1976
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261119-public-portal-redesign

## Goal

Governing GitHub Issue: [#1976](https://github.com/Oteryn/Oteryn-Platform/issues/1976).

Refine the Rust portal's shared chrome, home and start flow while keeping the selected Oteryn artwork and truthful service availability. This draft is an alternative to #1799.

## Acceptance criteria

- [x] Implement responsive PL/EN chrome, home, and native keyboard/touch disclosures.
- [x] Link to a real start section, and use registration delivery and abuse-policy decisions for the CTA.
- [ ] Qualify the exact candidate through native Rust, PostgreSQL and browser checks.
- [ ] Review the visual result with the owner and select one portal design before integration.

## Ownership

```yaml
owned_paths:
  - portal-rs/templates/components/shell-header.html
  - portal-rs/templates/pages/home.html
  - portal-rs/static/css/home.css
  - portal-rs/static/img/home/oteryn-gate.webp
  - portal-rs/crates/ui/**
  - portal-rs/crates/web/src/chrome.rs
  - portal-rs/crates/web/src/search/browser.cjs
  - portal-rs/bin/oteryn-portal/src/{main,registration_delivery,audit_l9_tests}.rs
  - portal-rs/e2e/specs/navigation.spec.mjs
modules:
  - ui
  - web composition
dependencies:
  - issue 1481 for published content and installer information
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-19T11:16:03Z
head: UNKNOWN
branch: codex/1976-public-portal-redesign
pr: none
status: validating
context_routes:
  - portal UI and web composition
owned_paths:
  - portal-rs/templates/components/shell-header.html
  - portal-rs/templates/pages/home.html
  - portal-rs/static/css/home.css
  - portal-rs/static/img/home/oteryn-gate.webp
  - portal-rs/crates/ui/**
  - portal-rs/crates/web/src/chrome.rs
  - portal-rs/crates/web/src/search/browser.cjs
  - portal-rs/bin/oteryn-portal/src/{main,registration_delivery,audit_l9_tests}.rs
  - portal-rs/e2e/specs/navigation.spec.mjs
proven:
  - Public registration refused with 503, and the existing client CTA reached an unpublished page.
  - Dedicated branch allocated to a single publishing writer for Issue 1976.
derived:
  - Portal chrome can use the existing registration policy and delivery preflight without changing admission.
unknown:
  - Exact candidate CI result and owner visual selection.
conflicts:
  - Draft PR 1799 is an alternative for the shared paths and is not part of this candidate.
first_failure:
  marker: local-rust-toolchain-dependencies
  evidence: Offline Cargo has no cached Askama dependency on the local executor.
rejected_hypotheses: []
changed_paths:
  - portal-rs/templates/components/shell-header.html
  - portal-rs/crates/ui/src/lib.rs
  - portal-rs/templates/pages/home.html
  - portal-rs/crates/ui/i18n/pl.ftl
  - portal-rs/crates/ui/i18n/en.ftl
  - portal-rs/static/css/home.css
  - portal-rs/crates/web/src/search/browser.cjs
  - portal-rs/e2e/specs/navigation.spec.mjs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/registration_delivery.rs
  - portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs
  - portal-rs/crates/web/src/chrome.rs
  - portal-rs/static/img/home/oteryn-gate.webp
  - docs/agents/tasks/active/OTERYN-20261119-public-portal-redesign.md
validation:
  - command: node --check on both edited browser check files
    result: PASS
    evidence: Both files passed JavaScript syntax validation.
  - command: cargo test --offline -p oteryn-ui
    result: BLOCKED
    evidence: Local Cargo cannot resolve the uncached Askama dependency offline.
blockers:
  - Exact candidate qualification is pending.
next_action: Inspect Portal RS CI and verify the rendered portal with functional PL and EN browser coverage.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

## Notes

The panorama is an original generated Oteryn illustration selected by the owner for this iteration; WebP is optimized for web delivery. Tibia artwork supplied as a reference is not bundled. Production activation remains scoped to its protected operations.
