---
task_id: OTERYN-20261105-portal-rs-ui-1
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# UI-1 — shared portal shell

Owner's UI-1 instruction governs scope; open Platform programme parent: https://github.com/Oteryn/Oteryn-Platform/issues/530. Base: main c75f1cea. Dedicated branch: codex/portal-rs-ui-1. Push branch only, no PR, no workflow edits. Jira mapping unknown; no Jira synchronization attempted.

## Ownership

```yaml
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261105-portal-rs-ui-1.md
modules:
  - shared-portal-shell
dependencies:
  - T39
blockers:
  - none
cross_repository_tasks:
  - none
```

## Implementation

Small `oteryn-ui` crate owns Chrome, registry, the existing shared Fluent catalogues and home/refusal rendering. Owning module crates register public entry routes from existing flags/mount prerequisites. Startup installs one outer Chrome extension adapter, reading existing browser sessions without writes or expiry refresh. Module page templates remain unchanged. Registration's existing route is the CTA; downloads exposes update APIs only and has no browser entry route. API-only modules, disabled configured adapters and routes requiring an unknown character/listing ID have no invented navigation destination. Existing form validation pages keep their field errors at 422; generic refusal responses use the shell.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-05T22:51:22Z
head: b77e290a903a005714feb2951492a6fa4da3329c
branch: claude/portal-rs-ui-1
pr: 1747
status: ready
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261105-portal-rs-ui-1.md
proven:
  - shared crate and module registration compile
  - enabled navigation, bilingual home, account areas and refusal tests pass
  - read-only PostgreSQL session presentation and binary startup tests pass
  - all exact acceptance commands pass on the implementation head above
derived: []
unknown: []
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
    evidence: final Rust sources formatted
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: exit 0, existing bazaar Cargo manifest warnings are unchanged
  - command: cargo test --locked --workspace
    result: PASS
    evidence: 993 passed, zero failed, 3 existing opt-in browser tests ignored; PostgreSQL 18.6 fresh cluster, max_connections=400, max_locks_per_transaction=512; RUST_TEST_THREADS=4
  - command: Playwright Chromium UI-1 live portal check
    result: PASS
    evidence: eight exact viewport screenshots, no horizontal overflow, native disclosure without JavaScript, skip-link keyboard focus, optional flags and dark theme checked
blockers:
  - none
next_action: archive this record; PR #1747 is integrated on main
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: same-repository PR #1747 used repository automatic source-branch deletion after squash merge
source_branch_evidence: PR #1747 merged as bd9898c4 and live branch lookup on 2026-11-11 confirms refs/heads/claude/portal-rs-ui-1 is absent
```

## Validation report

The implementation head above passed the exact three acceptance commands. The test cluster was a newly created, separate PostgreSQL 18.6 instance (`server_version_num=180006`) with the requested 400 connections and 512 locks per transaction. The three ignored tests are existing opt-in browser checks for payments, storefront and search; the requested UI-1 browser checks ran separately. Logs remain outside the repository under `/tmp/ui-tests-accepted.log`, `/tmp/ui-clippy-accepted-final.log` and `/tmp/ui1-browser-final.log`.

Playwright Chromium 156 rendered the real portal against PostgreSQL 18.6. Eight screenshots in `/tmp/ui1-{pl,en,404,503}-{1366,390}.png` have exact 1366×900 and 390×844 dimensions; none is committed. `/pl/` and `/en/` use grouped desktop navigation and a four-column card grid, with a one-column grid and native disclosure on mobile. Account controls wrap cleanly; the skip link focuses the main landmark. No page has horizontal overflow. The English 404 and Polish registration 503 retain the same header, navigation, account area, footer and localized back link. Registration's domain-specific unavailable message remains visible. An additional menu screenshot verifies the open native disclosure with JavaScript disabled. Default flags render 9 cards with login/online/puzzles absent; enabling login, online and puzzles renders the mounted links and 11 cards. System dark theme was also checked.

`Chrome` lives in `oteryn-ui`, independently of all module and web crates. Module-owned `navigation::register` functions call `Registry::section(enabled, section_id, fluent_key, route_suffix, group, footer)` using existing mount conditions; an existing absolute route such as `/account` is supported. A handler extracts `Option<Extension<Chrome>>`, defaults when absent, and supplies a `chrome` field to its Askama page extending `layout.html`. `oteryn_ui::error(&chrome, status)` renders shared localized refusals; `error_with_message` and `RefusalMessage` retain escaped domain detail. See the Page shell integration section in `portal-rs/static/design/README.md`.

An initial workspace run exposed an existing signing-key clock assertion that passed isolated and in the final fresh-cluster run; no gameauth code changed. Validation also caught and fixed lost registration refusal detail and an old rate-limit test expectation for surface.css. An intermediate debug build exhausted the 32 GB disk; duplicate generated executables and the first disposable test cluster were cleaned up. No test gate, production behavior or build profile was weakened.

Routes, authentication decisions, CSRF validation, enable flags, security headers/CSP and business logic are unchanged. Module page templates were not migrated, workflows were not edited and no PR was created. Registration is the home CTA because downloads currently exposes update APIs without a browser landing route.
