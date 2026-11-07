---
task_id: OTERYN-20261107-plat3-t94-wire-align
governing_issue: 1756
required_reads: []
search_first: []
optional_reads: []
---

# PLAT3-T94-WIRE-ALIGN

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1756

Owner authorized local implementation, checks and Conventional Commit; owner integrates.
No PR, deployment, workflow changes or Game writes. Game read-only contract baseline:
9668e0a04e1a631ebfc8752772102a1fd98ce8fa.

## Ownership

Owned paths: portal-rs/crates/minigames/src/league/**,
portal-rs/crates/minigames/Cargo.toml, portal-rs/Cargo.lock,
portal-rs/e2e/league-producer.mjs, portal-rs/e2e/specs/league.spec.mjs,
portal-rs/e2e/ENVIRONMENT.md. Branch: claude/portal-rs-t94-wire (PR 1760).
Live Issue and PR searches found no overlapping task; #1756 confirmed by owner.
Jira mapping unknown; synchronization pending coordinator.

## Acceptance

Exact canonical three-route Game JSON, dedicated verified mTLS SPKI pinned to
source oteryn:game:primary; no signature headers or cut feed. Fail closed on
unknown input, wrong identity, stale watermarks and unreconciled epochs.
Verify fixtures and required fmt/clippy/workspace tests plus league E2E.

## Evidence

All four copied fixture files now match raw Game URLs at the pinned SHA;
snapshots.json and plat3_authority.proto were already identical. Registry and
contract lock copies and hash pins updated to the same baseline.

Threat assumptions: only WebPKI-verified TLS1.3 client certificates reach the
identity gate. CA membership alone is insufficient; leaf DER SPKI must match the
dedicated configured pin. Browser headers cannot create the private peer extension.
Database-admitted snapshots retain the verified pin and are rechecked against
current trust. Native ANL records are Game-owned and never invented HTTP inputs.

No migration needed: existing additive inbox/audit schema stores canonical bytes
and the SPKI identity in historical signature/peer_certificate_sha256 columns.
Legacy signatures/leaf-certificate hashes do not match the new admission grammar.
No existing migrations or workflows were edited.

Browser league E2E: 8 passed (2.2m), en/pl x desktop/mobile, real TLS1.3 and
CA-valid alternate-client rejection, worker/public standings, stale-watermark 503.
fmt and clippy -D warnings passed. Workspace validation switched to the dedicated
task database portal_e2e_t94_wire_tests_20261107 with RUST_TEST_THREADS=4:
initial shared-environment attempts encountered PostgreSQL lock exhaustion and
preexisting public-schema wallet tables without their migration record. No
application code or migrations were changed to accommodate that environment.
Final workspace result: exit 0; 1097 passed, 0 failed, 3 existing ignored
browser-only tests. All 49 minigames tests passed, including the new Game-wire,
pin-refusal, equal-pair conflict and epoch reconciliation regressions.
PostgreSQL required local max_locks_per_transaction=1024 for this repository's
large parallel schema migrations; the temporary override was reset after tests.
Task test database dropped; E2E launcher stopped and its two databases dropped;
disposable E2E certificates/configuration removed. Shared local containers,
started from stopped state for validation, returned to stopped state. Shared
persistent volumes and reusable dependency caches are preserved.

Source branch disposition: retain for owner integration; no autonomous publication.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-07T12:05:00Z
head: 541af097dd9e0c0031afb4fee269050ac70096b3
branch: claude/portal-rs-t94-wire
pr: 1760
status: ready
context_routes:
  - security
  - testing
owned_paths:
  - portal-rs/crates/minigames/src/league/**
  - portal-rs/crates/minigames/Cargo.toml
  - portal-rs/Cargo.lock
  - portal-rs/e2e/league-producer.mjs
  - portal-rs/e2e/specs/league.spec.mjs
  - portal-rs/e2e/ENVIRONMENT.md
proven:
  - all four copied fixtures byte-identical to Game 9668e0a04e1a631ebfc8752772102a1fd98ce8fa
  - browser league E2E 8 passed
  - fmt and workspace clippy pass
derived:
  - no database migration required
unknown:
  - owner integration remains pending
conflicts: []
first_failure:
  marker: local-test-database
  evidence: shared schema had preexisting wallet tables and concurrent migration lock exhaustion; isolated task database provisioned
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/minigames/src/league/**
  - portal-rs/crates/minigames/Cargo.toml
  - portal-rs/Cargo.lock
  - portal-rs/e2e/league-producer.mjs
  - portal-rs/e2e/specs/league.spec.mjs
  - portal-rs/e2e/ENVIRONMENT.md
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: exit 0, no output
  - command: cargo clippy -q --workspace --all-targets -- -D warnings
    result: PASS
    evidence: exit 0, no output
  - command: cargo test -q --workspace
    result: PASS
    evidence: exit 0; 1097 passed, 0 failed, 3 existing ignored browser tests; real PostgreSQL enabled
  - command: node run.mjs /tmp/t94-e2e/env.json test specs/league.spec.mjs
    result: PASS
    evidence: 8 passed (2.2m), real dedicated mTLS listener
blockers:
  - none
next_action: PR 1760 merges through the Merge Queue; then archive this record
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: same-repository PR 1760 is open; disposition becomes terminal at merge
source_branch_evidence: PR 1760 for governing Issue 1756
```
