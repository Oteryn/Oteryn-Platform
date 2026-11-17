---
task_id: OTERYN-20261117-portal-rs-device-ticket-clock
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-portal-rs-device-ticket-clock

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Portal RS CI test shard 1 on PR 1898 head 17269518 failed
`devices::tests::database_native_normal_ticket_consumes_oauth_but_device_continues`
(`rotate(..., Purpose::Ticket)` not Ok). Remembered-device rotation reads its
clock `at` before the native ticket is issued; the issuer stamps
`expires_at = clock_timestamp() + 60 s`. When a second boundary falls between
the two reads, `expires_at == at + 61` and the bound `expires_at <= at + 60`
refuses a valid ticket with Unavailable. The contract bounds the ticket
lifetime from its issuance ("strictly after issuance and no later than 60
seconds"). Fix: read the clock again after issuance and bound the upper end
from it; keep `expires_at > at`, `1..=60` for `expires_in` and the 60-second
bound itself. The issuer is unchanged.

## Acceptance criteria

- [x] Red: `database_ticket_expiry_is_bounded_from_issuance` fails with Unavailable on origin/main bb2f6c07 (issuer stamping in a later second than the rotation clock)
- [x] Real issuer: a temporary wait for the next database second after the rotation clock read makes `database_native_normal_ticket_consumes_oauth_but_device_continues` fail at its `rotate(..., Purpose::Ticket)` assert with the old bound and pass with the new one
- [x] Green: the same test passes with the post-issuance bound; a 120-second lifetime is still refused
- [x] `cargo test -p oteryn-gameauth --lib` 166 passed; fmt and clippy clean
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/gameauth/src/devices.rs
  - portal-rs/crates/gameauth/src/devices_tests.rs
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-device-ticket-clock.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-e2e-hosted.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-e2e-hosted.md
modules:
  - gameauth
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T03:50:00Z
head: bb2f6c07f2aa9d2fa820750753aa7daba376b9e4
branch: claude/happy-davinci-ijz3wy-devclock
pr: 1901
status: validating
context_routes: []
owned_paths:
  - portal-rs/crates/gameauth/src/devices.rs
  - portal-rs/crates/gameauth/src/devices_tests.rs
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-device-ticket-clock.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-e2e-hosted.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-e2e-hosted.md
proven:
  - The only other issue_on_connection caller (tickets::issue) has no expiry bound against an earlier clock
derived: []
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: devices_tests.rs rotate Purpose::Ticket not Ok
  evidence: Portal RS CI test shard 1 on PR 1898 head 17269518 (rerun)
rejected_hypotheses:
  - Issuer TTL change (issuer keeps 60 s from clock_timestamp)
changed_paths:
  - portal-rs/crates/gameauth/src/devices.rs
  - portal-rs/crates/gameauth/src/devices_tests.rs
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-device-ticket-clock.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-e2e-hosted.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-e2e-hosted.md
validation:
  - command: cargo test -p oteryn-gameauth --lib database_native_normal_ticket_consumes_oauth_but_device_continues (temporary second-advance wait after the rotation clock, uncommitted)
    result: PASS
    evidence: old bound fails at the rotate Purpose::Ticket assert; new bound 1 passed
  - command: cargo test -p oteryn-gameauth --lib (PostgreSQL 18, with fix)
    result: PASS
    evidence: 166 passed
  - command: cargo fmt --all --check; cargo clippy -p oteryn-gameauth --all-targets -- -D warnings
    result: PASS
    evidence: fmt clean; clippy finished with no warnings
blockers:
  - none
next_action: wait for Portal RS CI on PR 1901
```
