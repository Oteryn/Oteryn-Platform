---
task_id: OTERYN-20261108-portal-rs-home-template
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# Home page: owner fantasy template 1:1

Owner instruction 2026-11-08: port the owner-approved homepage template (art + layout from the owner's mockup, adapted to Platform modules) into portal-rs 1:1 and deploy it to the PC docker testenv. Programme parent: https://github.com/Oteryn/Oteryn-Platform/issues/530. Base: main a2c93448. Branch: claude/portal-rs-ui-2-fantasy-shell. Jira mapping unknown; no Jira synchronization attempted.

## Ownership

```yaml
owned_paths:
  - portal-rs/templates/pages/home.html
  - portal-rs/templates/layout.html
  - portal-rs/static/css/home.css
  - portal-rs/static/img/home/**
  - portal-rs/static/fonts/**
  - portal-rs/crates/ui/**
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/surface.rs
  - docs/agents/tasks/active/OTERYN-20261108-portal-rs-home-template.md
modules:
  - shared-portal-shell
dependencies:
  - UI-1
blockers:
  - none
cross_repository_tasks:
  - none
```

## Implementation

The template background (owner art) is a static asset; every text sits at the template's 1672x941 coordinates, moved from inline style attributes into `static/css/home.css` (strict CSP keeps `style-src 'self'`, no inline style or script). Navigation dropdowns, footer links, account bar and CTA render only mounted modules (`Chrome::link`, `Chrome::first_link`); signed-in visitors get account + POST logout with CSRF. Texts are Fluent keys (`stage-*`) in PL and EN. Fonts Spectral and PT Sans are self-hosted (OFL). Status, news and ranking blocks still show the template's sample copy; wiring them to live read models is the next slice.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-08T00:40:00Z
head: b4518fe3e709c12591cd99897ad3786a88ab351f
branch: claude/portal-rs-ui-2-fantasy-shell
pr: 1782
status: validating
context_routes: []
owned_paths:
  - portal-rs/templates/pages/home.html
  - portal-rs/static/css/home.css
proven:
  - oteryn-ui and oteryn-web tests pass locally
  - live local portal screenshots match the template at 1672x941 (PL) and 1440x810 (EN), no horizontal overflow
derived: []
unknown:
  - live data for status, news and ranking blocks
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/**
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: clean
  - command: cargo clippy -p oteryn-ui -p oteryn-web --all-targets -- -D warnings
    result: PASS
    evidence: exit 0
  - command: cargo test -p oteryn-ui -p oteryn-web
    result: PASS
    evidence: ui 3 passed, web 24 passed (1 existing ignored)
blockers:
  - none
next_action: CI on PR #1782; PC docker testenv deploy of this branch; wire live status, news and ranking blocks
```
