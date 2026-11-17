---
task_id: OTERYN-20261117-channel-probe
governing_issue: 1473
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-channel-probe

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1473.
The owner decided on 2026-11-17 that the channel picker shows a ping per
channel. A v1 contract (ACCEPTED by the client on 2026-11-17), separate from the client directory, for the
probe list `GET /v1/client/channel-probes` on the portal and the latency
target `GET /v1/probe` on each regional edge, plus byte-exact fixtures. No
handler, route, migration or flag; the list route is a follow-up to T36b.

## Acceptance criteria

- [x] Contract `OTERYN_V2_CLIENT_CHANNEL_PROBE_CONTRACT.md` v1 ACCEPTED (probe list, probe target, no route keys, security notes)
- [x] Fixtures probes-ok, rejected-route-key, rejected-http-url, rejected-query and probe-target-204 with manifest and README
- [x] Manifest digests checked (`native_client_fixture_manifests_match_their_files`)
- [x] Each vector matches its declared client outcome (`client_channel_probe_vectors_match_their_declared_outcome`)
- [x] Portal RS CI green on the PR head
- [x] Game client session confirmed the contract (warm-up plus 3 timed HEAD, 2 s timeout)

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-channel-probe.md
  - docs/contracts/OTERYN_V2_CLIENT_CHANNEL_PROBE_CONTRACT.md
  - docs/contracts/fixtures/client-channel-probe-v1/**
  - portal-rs/crates/gameauth/src/oauth/oauth_fixture_tests.rs
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
modules:
  - gameauth
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - Oteryn-Game client session confirms the probe contract and adds a parser (coordinator)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T18:00:00Z
head: d2e54f2ebe2d8be5ac592bba355c4ba2d13b5c57
branch: claude/happy-davinci-ijz3wy-channel-probe
pr: 1939
status: completed
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-channel-probe.md
  - docs/contracts/OTERYN_V2_CLIENT_CHANNEL_PROBE_CONTRACT.md
  - docs/contracts/fixtures/client-channel-probe-v1/**
  - portal-rs/crates/gameauth/src/oauth/oauth_fixture_tests.rs
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
proven:
  - The outcome test fails when a manifest outcome is changed (probes-rejected-http-url and probes-rejected-query each set to accept, one at a time)
  - probes-ok lists exactly the channel ids of client-directory-v1 directory-anonymous.json on two distinct probe URLs
derived:
  - A portal-served probe would measure latency to the portal, so the target lives on the regional edge
unknown:
  - Game client confirmation of the contract
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Per-channel probe on the portal keyed by channel_ref; it measures the portal, not the region
  - probe_url inside the client directory; the directory carries no route keys
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-channel-probe.md
  - docs/contracts/OTERYN_V2_CLIENT_CHANNEL_PROBE_CONTRACT.md
  - docs/contracts/fixtures/client-channel-probe-v1/README.md
  - docs/contracts/fixtures/client-channel-probe-v1/manifest.json
  - docs/contracts/fixtures/client-channel-probe-v1/probes-ok.json
  - docs/contracts/fixtures/client-channel-probe-v1/probes-rejected-route-key.json
  - docs/contracts/fixtures/client-channel-probe-v1/probes-rejected-http-url.json
  - docs/contracts/fixtures/client-channel-probe-v1/probes-rejected-query.json
  - docs/contracts/fixtures/client-channel-probe-v1/probe-target-204.body
  - portal-rs/crates/gameauth/src/oauth/oauth_fixture_tests.rs
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
validation:
  - command: cargo test --locked -p oteryn-gameauth --lib oauth_fixture
    result: PASS
    evidence: 5 passed, 0 failed
  - command: pg_token_endpoint_matches_native_oauth_ticket_fixture
    result: NOT_RUN
    evidence: printed SKIPPED, PORTAL_TEST_DATABASE_URL unset; untouched by this change
  - command: cargo fmt --all --check; cargo clippy --locked -p oteryn-gameauth --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: contract and fixtures only; no route, handler or flag changes
blockers:
  - none
next_action: none; delivered by PR 1939 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1939 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1939 merged as d2e54f2ebe2d8be5ac592bba355c4ba2d13b5c57
```
