---
task_id: OTERYN-20261117-newhaven-vocation
governing_issue: 1937
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-newhaven-vocation

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1937.
Part 1: bootstrap-intent producer contract v4 binds `requested_vocation`
(Knight, Paladin, Sorcerer, Druid, Monk) so Newhaven characters get the
vocation chosen at creation. Field names were agreed with the Game server lane
on 2026-11-17. Part 2 (character-command `create` and the web form) is a later
PR under the same Issue.

## Acceptance criteria

- [x] Contract v4 documents `requested_vocation`; v1-v3 stored intents and v3 read requests fail closed
- [x] Issuer validates the vocation before any allocation and binds it into the conflict check, column and hashed JSON
- [x] CLI `requestedVocation` is required; the seed issues a vocation
- [x] Fixtures `character-bootstrap-intent-v4/` match the serializer and validator byte for byte
- [x] Additive nullable migration with a CHECK on the five values
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-newhaven-vocation.md
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
  - docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md
  - docs/contracts/fixtures/character-bootstrap-intent-v4/**
  - portal-rs/crates/characters/src/bootstrap/**
  - portal-rs/bin/oteryn-portal/src/bootstrap_intent.rs
  - portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs
  - portal-rs/bin/oteryn-portal/src/seed.rs
  - portal-rs/bin/oteryn-portal/src/seed_tests.rs
  - portal-rs/e2e/README.md
  - portal-rs/e2e/seed-plan.example.json
  - portal-rs/migrations/20261117160500_bootstrap_intent_v4_vocation.sql
modules:
  - characters
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - Oteryn-Game consumer moves CONTRACT_VERSION 3 to 4 (server lane)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T16:20:00Z
head: 5b67d4aed872106010dbef8ac0d27692f3958920
branch: claude/happy-davinci-ijz3wy-vocation
pr: 1938
status: completed
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-newhaven-vocation.md
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
  - docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md
  - docs/contracts/fixtures/character-bootstrap-intent-v4/**
  - portal-rs/crates/characters/src/bootstrap/**
  - portal-rs/bin/oteryn-portal/src/bootstrap_intent.rs
  - portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs
  - portal-rs/bin/oteryn-portal/src/seed.rs
  - portal-rs/bin/oteryn-portal/src/seed_tests.rs
  - portal-rs/e2e/README.md
  - portal-rs/e2e/seed-plan.example.json
  - portal-rs/migrations/20261117160500_bootstrap_intent_v4_vocation.sql
proven:
  - Game server lane confirmed v4, requested_vocation spelling, newhaven-starter-r1 and Monk at creation (2026-11-17)
  - The fixture test fails when one byte of intent.monk.json changes
derived:
  - Game consumer at e2152d050038cecdb3728c222af2ac184e1cac5a still pins contract version 3 and refuses v4 until its PR lands
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Keeping v3 readable; the contract retires each earlier version on a bump
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-newhaven-vocation.md
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
  - docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md
  - docs/contracts/fixtures/character-bootstrap-intent-v4/README.md
  - docs/contracts/fixtures/character-bootstrap-intent-v4/intent.druid.json
  - docs/contracts/fixtures/character-bootstrap-intent-v4/intent.knight.json
  - docs/contracts/fixtures/character-bootstrap-intent-v4/intent.monk.json
  - docs/contracts/fixtures/character-bootstrap-intent-v4/intent.paladin.json
  - docs/contracts/fixtures/character-bootstrap-intent-v4/intent.sorcerer.json
  - docs/contracts/fixtures/character-bootstrap-intent-v4/manifest.json
  - docs/contracts/fixtures/character-bootstrap-intent-v4/rejected.vocation-lower-case.json
  - docs/contracts/fixtures/character-bootstrap-intent-v4/rejected.vocation-missing.json
  - docs/contracts/fixtures/character-bootstrap-intent-v4/rejected.vocation-none.json
  - portal-rs/bin/oteryn-portal/src/bootstrap_intent.rs
  - portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs
  - portal-rs/bin/oteryn-portal/src/seed.rs
  - portal-rs/bin/oteryn-portal/src/seed_tests.rs
  - portal-rs/crates/characters/src/bootstrap/issue.rs
  - portal-rs/crates/characters/src/bootstrap/issue_tests.rs
  - portal-rs/crates/characters/src/bootstrap/mod.rs
  - portal-rs/crates/characters/src/bootstrap/tests.rs
  - portal-rs/crates/characters/src/bootstrap/wire.rs
  - portal-rs/e2e/README.md
  - portal-rs/e2e/seed-plan.example.json
  - portal-rs/migrations/20261117160500_bootstrap_intent_v4_vocation.sql
validation:
  - command: cargo test --locked -p oteryn-characters
    result: PASS
    evidence: 105 + 1 passed, 0 failed, PostgreSQL 18 set
  - command: cargo test --locked -p oteryn-portal
    result: PASS
    evidence: 217 unit passed, 1 existing ignore; integration 5 passed
  - command: cargo test --locked -p oteryn-db; cargo fmt --all --check; cargo clippy -p oteryn-characters -p oteryn-portal --all-targets -- -D warnings
    result: PASS
    evidence: 12 passed; fmt and clippy exit 0
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: operator CLI and private mTLS read only; covered by the PostgreSQL CLI and reader tests
blockers:
  - none
next_action: none; delivered by PR 1938 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1938 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1938 merged as 5b67d4aed872106010dbef8ac0d27692f3958920
```
