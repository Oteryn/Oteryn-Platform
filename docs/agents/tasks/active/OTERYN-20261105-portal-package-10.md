---
task_id: OTERYN-20261105-portal-package-10
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# Portal package 10 integration

## Goal

Owner-authorized integration of T58, T59, T60, T61, T62, T63, T73, T75, T76, T77, T79 and T80, in that order, onto current main. Programme parent: https://github.com/Oteryn/Oteryn-Platform/issues/530 (open); the current owner request supplies the package scope. No PR is requested.

## Acceptance criteria

- Preserve module behavior and both sides of shared registrations.
- Unique migration versions; preserve main migrations.
- Formatting, locked Clippy with warnings denied, locked tests against PostgreSQL 18.6, and legacy token ratchet.
- Normal branch push and remote head verification; no workflow edits.

## Ownership

```yaml
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261105-portal-package-10.md
modules:
  - portal package 10 integration
dependencies:
  - accepted contracts on main
blockers:
  - none
cross_repository_tasks:
  - none
```

## Conflict resolutions

All paths below are relative to `portal-rs/` except the contract document.

| Merge | Conflicted files and resolution |
| --- | --- |
| T58 | `docs/contracts/OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md`: retained main's revision 2; the module carried an older contract copy. |
| T59 | `Cargo.lock`: retained pre-merge lock and regenerated with Cargo. `bin/oteryn-portal/Cargo.toml`, `bin/oteryn-portal/src/main.rs`: retained existing dependencies/routes and added Bazaar characters. `crates/bazaar/Cargo.toml`: combined dependency requirements, promoting serde/serde_json from test-only to runtime dependencies and declaring each once. `crates/bazaar/src/lib.rs`: retained game and added characters. A union-resolution error was detected and fully repaired during T60 resolution. |
| T60 | `Cargo.lock`: regenerated from pre-merge lock. `bin/oteryn-portal/Cargo.toml`: retained existing dependencies and one Bazaar entry. `bin/oteryn-portal/src/main.rs`: retained all main/characters routes and added entity mount and item routes, with one listener/test/module tail. `crates/bazaar/Cargo.toml`: combined features and test dependencies without duplicate tables. `crates/bazaar/src/lib.rs`: retained game/characters and added items. `crates/wiki/Cargo.toml`, `crates/wiki/src/lib.rs`: retained existing dependencies/modules and added entities, with one world dependency and domain declaration. |
| T61 | `crates/bazaar/Cargo.toml`: retained equivalent serde requirements (promoted to runtime during final validation). `crates/bazaar/src/lib.rs`: retained all modules and added search/export. |
| T62 | `Cargo.lock`: regenerated from pre-merge lock. `crates/bazaar/Cargo.toml`: retained compatible fluent/unic requirements including macros. `crates/bazaar/src/lib.rs`: retained all modules and added notify/export. |
| T63 | `crates/bazaar/src/lib.rs`: retained all modules and added stats. |
| T73 | `Cargo.lock`: regenerated from pre-merge lock. `Cargo.toml`: retained hirelings and added playerdata. `bin/oteryn-portal/Cargo.toml`, `bin/oteryn-portal/src/main.rs`: retained existing dependencies/routes and added playerdata/hunts. |
| T75 | `crates/playerdata/Cargo.toml`, `crates/playerdata/src/lib.rs`: combined hunts/drops, test dependency and language macros. The automatically merged lock had duplicate playerdata packages; restored pre-merge lock and regenerated. |
| T76 | `Cargo.lock`: regenerated from pre-merge lock. `Cargo.toml`, `bin/oteryn-portal/Cargo.toml`, `crates/characters/src/lib.rs`, `crates/store/Cargo.toml`: retained existing declarations absent from the older branch. `bin/oteryn-portal/src/main.rs`: retained hunts and added items, preserving a complete pool argument and closing call for each. `crates/playerdata/Cargo.toml`, `crates/playerdata/src/lib.rs`: combined modules and added async-trait, sha2, time and HTTP test support. `crates/world/Cargo.toml`: retained existing dependencies and added tempfile. |
| T77 | `crates/playerdata/Cargo.toml`, `crates/playerdata/src/lib.rs`: retained existing requirements/modules, added progress and serde_json raw_value. Restored pre-merge lock and regenerated to avoid duplicate packages. |
| T79 | `Cargo.lock`: regenerated from pre-merge lock. `bin/oteryn-portal/Cargo.toml`: retained existing playerdata entry. `bin/oteryn-portal/src/main.rs`: retained routes and added dashboard. `crates/playerdata/Cargo.toml`, `crates/playerdata/src/lib.rs`: retained modules, added dashboard/reminders, promoted db dependency to runtime. |
| T80 | `Cargo.lock`: regenerated from pre-merge lock. `Cargo.toml`, `crates/characters/src/lib.rs`, `crates/store/Cargo.toml`, `crates/world/Cargo.toml`: retained existing main declarations. `bin/oteryn-portal/Cargo.toml`: combined requirements with one identity/playerdata entry and added sha2. `bin/oteryn-portal/src/main.rs`: combined routes and sessions setup, declaring login route once. `crates/identity/src/lib.rs`: retained abuse/security wizard/login and added sessions, declaring login once. `crates/playerdata/Cargo.toml`, `crates/playerdata/src/lib.rs`: added identity, SQLx derive/time features, and logins while preserving all previous modules. |

No i18n or permission-catalog conflicts occurred. No module was dropped. Migration numbers are unique without renumbering. No SQLx offline-data directory exists.

Main already contains `fix(portal-rs): pin the gateway TLS provider to ring` (c4c6715d), explicitly passing the ring provider to the client builder. Preserve this minimal upstream fix and verify it workspace-wide; no additional provider mutation is needed unless tests fail.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-05T00:00:00Z
head: 9ec36d5c
branch: codex/portal-rs-pkg10
pr: none
status: validating
context_routes:
  - testing
owned_paths:
  - portal-rs/**
proven:
  - all twelve requested module branches merged in order
  - migration versions unique
  - no SQLx offline data
  - legacy token ratchet passed
derived: []
unknown:
  - final acceptance results
  - Jira mapping unavailable; synchronization pending
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
    evidence: completed successfully
  - command: cargo clippy --locked --all-targets -- -D warnings
    result: NOT_RUN
    evidence: compiling
blockers:
  - none
next_action: finish acceptance and verify normal branch push
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requested a pushed integration branch without a PR; retain for owner review
source_branch_evidence: remote verification pending
```

Product runtime E2E: NOT_APPLICABLE to this assembly task; existing module tests provide behavior verification. No product redesign or deployment is authorized.
