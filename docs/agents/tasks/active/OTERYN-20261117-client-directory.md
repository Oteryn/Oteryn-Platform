---
task_id: OTERYN-20261117-client-directory
governing_issue: 1473
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-client-directory

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1473.
Option (a) for DirectoryChannel, confirmed by the programme coordinator after
the owner lifted the hold on 2026-11-17: a v1 contract for
`GET /v1/client/directory` with channel `pvp_type` and `region`, plus
byte-exact fixtures. No handler, migration or flag; the endpoint is T36b,
a separate task.

## Acceptance criteria

- [x] Contract `OTERYN_V2_CLIENT_DIRECTORY_CONTRACT.md` v1 (shape, limits, no route keys, owner-only characters)
- [x] Fixtures anonymous, owner, rejected-route-key, unknown-pvp_type with manifest and README
- [x] Manifest digests checked (`native_client_fixture_manifests_match_their_files`)
- [x] Each vector matches its declared client outcome (`client_directory_vectors_match_their_declared_outcome`)
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-client-directory.md
  - docs/contracts/OTERYN_V2_CLIENT_DIRECTORY_CONTRACT.md
  - docs/contracts/fixtures/client-directory-v1/**
  - portal-rs/crates/gameauth/src/oauth/oauth_fixture_tests.rs
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
modules:
  - gameauth
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - Oteryn-Game platform-client reads pvp_type and region (coordinator)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T15:00:00Z
head: 5510ab0a840a009eecdab01fd988823a258d1f30
branch: claude/happy-davinci-ijz3wy-client-directory
pr: none
status: validating
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-client-directory.md
  - docs/contracts/OTERYN_V2_CLIENT_DIRECTORY_CONTRACT.md
  - docs/contracts/fixtures/client-directory-v1/**
  - portal-rs/crates/gameauth/src/oauth/oauth_fixture_tests.rs
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
proven:
  - Game consumer at e2152d050038cecdb3728c222af2ac184e1cac5a reads character wire keys id and name and rejects 12 route keys at any depth
  - The outcome test fails when a manifest outcome is changed (unknown-pvp-type set to accept)
derived:
  - The pinned Game parser ignores pvp_type and region, so v1 bodies stay readable by today's client
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Wire keys character_ref and display_name; those are Rust field names in platform-contracts
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-client-directory.md
  - docs/contracts/OTERYN_V2_CLIENT_DIRECTORY_CONTRACT.md
  - docs/contracts/fixtures/client-directory-v1/README.md
  - docs/contracts/fixtures/client-directory-v1/manifest.json
  - docs/contracts/fixtures/client-directory-v1/directory-anonymous.json
  - docs/contracts/fixtures/client-directory-v1/directory-owner.json
  - docs/contracts/fixtures/client-directory-v1/directory-rejected-route-key.json
  - docs/contracts/fixtures/client-directory-v1/directory-unknown-pvp-type.json
  - portal-rs/crates/gameauth/src/oauth/oauth_fixture_tests.rs
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
validation:
  - command: cargo test -p oteryn-gameauth --lib oauth_fixture
    result: PASS
    evidence: 4 passed, 0 failed
  - command: cargo fmt --all --check; cargo clippy -p oteryn-gameauth --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: contract and fixtures only; no route, handler or flag changes
blockers:
  - none
next_action: Portal RS CI on the PR head; archive this packet once the PR lands.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
