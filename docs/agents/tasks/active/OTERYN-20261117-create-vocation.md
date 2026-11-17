---
task_id: OTERYN-20261117-create-vocation
governing_issue: 1937
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-create-vocation

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1937.
Part 2: the character-command create intent moves to v2 with `requested_sex`
and `requested_vocation` after `requested_name`, and the web creation form
requires both, so no path creates a Newhaven character without a vocation.
Member order and value sets agreed with the Game server lane on 2026-11-17.

## Acceptance criteria

- [x] Create intent v2 wire order and value sets match the Game agreement; other command families unchanged
- [x] Stored v1 creates are not served by pending or read
- [x] The web form requires sex and vocation; bad or missing values create no command
- [x] Fixtures `character-create-v2/` match the serializer and validator byte for byte
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-create-vocation.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/contracts/fixtures/character-create-v2/**
  - portal-rs/crates/characters/src/commands/**
  - portal-rs/crates/characters/src/create_web/**
  - portal-rs/crates/characters/src/restore/tests.rs
modules:
  - characters
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - Oteryn-Game create consumer moves contract_version 1 to 2 (server lane)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T16:40:00Z
head: 07fe2ba851f94b526e39810e15a7e6e2811ea618
branch: claude/happy-davinci-ijz3wy-create-vocation
pr: 1940
status: validating
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-create-vocation.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/contracts/fixtures/character-create-v2/**
  - portal-rs/crates/characters/src/commands/**
  - portal-rs/crates/characters/src/create_web/**
  - portal-rs/crates/characters/src/restore/tests.rs
proven:
  - Game server lane gave the CreateWire member order at e2152d050038cecdb3728c222af2ac184e1cac5a and agreed the v2 insertion point
  - Dropping the two binding lines turns six tests red
derived:
  - Game refuses create v2 until its consumer PR lands, so create flags stay off until then
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - A migration for sex and vocation columns; both live in binding_json and intent_json
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-create-vocation.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/contracts/fixtures/character-create-v2/README.md
  - docs/contracts/fixtures/character-create-v2/create-intent-druid.json
  - docs/contracts/fixtures/character-create-v2/create-intent-knight.json
  - docs/contracts/fixtures/character-create-v2/create-intent-monk.json
  - docs/contracts/fixtures/character-create-v2/create-intent-paladin.json
  - docs/contracts/fixtures/character-create-v2/create-intent-sorcerer.json
  - docs/contracts/fixtures/character-create-v2/manifest.json
  - docs/contracts/fixtures/character-create-v2/rejected-sex-lowercase.json
  - docs/contracts/fixtures/character-create-v2/rejected-sex-missing.json
  - docs/contracts/fixtures/character-create-v2/rejected-vocation-lowercase.json
  - docs/contracts/fixtures/character-create-v2/rejected-vocation-missing.json
  - portal-rs/crates/characters/src/commands/README.md
  - portal-rs/crates/characters/src/commands/create_v2_tests.rs
  - portal-rs/crates/characters/src/commands/mod.rs
  - portal-rs/crates/characters/src/commands/storage.rs
  - portal-rs/crates/characters/src/commands/tests.rs
  - portal-rs/crates/characters/src/commands/wire.rs
  - portal-rs/crates/characters/src/create_web/README.md
  - portal-rs/crates/characters/src/create_web/en.ftl
  - portal-rs/crates/characters/src/create_web/http.rs
  - portal-rs/crates/characters/src/create_web/mod.rs
  - portal-rs/crates/characters/src/create_web/page.rs
  - portal-rs/crates/characters/src/create_web/pl.ftl
  - portal-rs/crates/characters/src/create_web/service.rs
  - portal-rs/crates/characters/src/create_web/tests.rs
  - portal-rs/crates/characters/src/restore/tests.rs
validation:
  - command: cargo test --locked -p oteryn-characters
    result: PASS
    evidence: 104 + 1 passed, 0 failed, PostgreSQL 18 set
  - command: cargo test --locked -p oteryn-web; cargo test --locked -p oteryn-db; cargo test --locked -p oteryn-internal-api --test contracts
    result: PASS
    evidence: 61 passed (1 existing ignore); 12 passed; 11 passed
  - command: cargo fmt --all --check; cargo clippy --locked -p oteryn-characters -p oteryn-portal -p oteryn-web --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: create flags stay off until the Game consumer accepts v2; form-to-intent path covered by the web PostgreSQL tests
blockers:
  - none
next_action: wait for Portal RS CI on the PR head
```
