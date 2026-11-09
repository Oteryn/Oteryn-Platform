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
  - portal-rs/crates/web/src/session_tests.rs
  - portal-rs/crates/community/src/home.rs
  - portal-rs/crates/community/src/lib.rs
  - portal-rs/crates/community/src/worlds.rs
  - portal-rs/crates/community/src/rankings.rs
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/community/tests/rankings.rs
  - portal-rs/crates/community/Cargo.toml
  - portal-rs/crates/content/src/news.rs
  - portal-rs/crates/content/src/news_tests.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
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
updated_at: 2026-11-09T18:46:28Z
head: b4518fe3e709c12591cd99897ad3786a88ab351f
branch: claude/portal-rs-ui-2-fantasy-shell
pr: 1782
status: validating
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - portal-rs/templates/pages/home.html
  - portal-rs/static/css/home.css
proven:
  - oteryn-ui and oteryn-web tests pass locally
  - responsive home (390, 900, 1600, 1920 px) without horizontal overflow, rendered from live local read models
  - status from fresh world_online evidence, news from published CMS rows, ranking rows each pass the current profile privacy decision (DB test home_top_skips_hidden_and_fails_closed)
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
next_action: "Archive this predecessor task with verified PR #1782 closeout evidence; the new visual-polish work is tracked by Issue #1798."
```

## Lifecycle reconciliation — 2026-11-09

Live GitHub readback confirms PR #1782 was merged on 2026-11-08 at `0571e5b73b3b30690861b38ba7bf5b39b215117f`. The governing programme Issue #530 remains open. This packet now uses the existing `archive_pending` transition and no longer claims active implementation ownership. Final archival and source-branch disposition remain to be recorded; this change does not claim production deployment or requalify the historical head and test entries above. The follow-up visual polish has its own governing Issue #1798 and task packet.
