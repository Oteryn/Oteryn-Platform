---
task_id: OTERYN-20261105-portalrs-local-e2e
governing_issue: 530
required_reads:
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
search_first:
  - portal-rs/crates/identity
optional_reads: []
---

# Local portal-rs end-to-end qualification

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530 (live-read open).
Owner task in chat authorizes local test data, isolated portalrs containers and local fix commits only; forbids GitHub writes, publication and existing-stack changes.

## Goal and acceptance

Exercise registration, email verification, login/logout, MFA, recovery, abuse/CSRF and account reads in the isolated local environment. Produce `/home/mole/oteryn-cache/portalrs-e2e-report.md` and reproducible scripts under `portalrs-e2e/`.

## Ownership

Owned: this task record, bounded fixes within `portal-rs/`, external test scripts and `oteryn-testenv-portalrs*` resources only. One writer, local branch `portalrs-e2e-fixes`. Existing pkg6 packet is a ready integration record, not an active writer; live open PRs cover architecture draft and retained salvage, with no active implementation overlap found. No external repository inspection. Jira synchronization pending; this worker does not load Jira.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-05T22:36:32Z
head: 4ea9b99e8eaaba01c71ffd19bcad922735144862
branch: none
pr: none
status: waiting
context_routes:
  - testing
  - auth-identity
  - execution-resources
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261105-portalrs-local-e2e.md
proven:
  - isolated portal and local PostgreSQL run
  - live Issue 530 remains open
  - user prohibits all GitHub writes
unknown:
  - owner decision on exact anonymous-volume cleanup
conflicts: []
derived: []
first_failure:
  marker: email.verify unknown worker job kind
  evidence: local browser and outbox state; fixed in a560970
rejected_hypotheses: []
changed_paths:
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/registration_delivery.rs
  - portal-rs/crates/web/src/routes/verify.rs
  - portal-rs/crates/identity/tests/registration.rs
  - portal-rs/crates/web/tests/ratelimit.rs
validation:
  - command: Playwright browser E2E in PL and EN
    result: PASS
    evidence: 24 identity cases pass; two account sweeps retain FAIL for disabled-surface 503s; all 42 GET results in external report; no final 500
  - command: cargo test --locked --release -- --test-threads=4
    result: PASS
    evidence: 991 passed, zero failed, three existing ignored; workspace-final.log
  - command: cargo clippy --locked --release --all-targets -- -D warnings
    result: PASS
    evidence: clippy-final.log
  - command: cargo fmt --all --check and git diff --check
    result: PASS
    evidence: final material candidate before evidence commit
blockers:
  - explicit owner decision required for anonymous checks volume outside permitted prefix
next_action: obtain owner decision to remove exact disposable anonymous volume or retain it
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requests local fix commits for review; no push or PR authorized
source_branch_evidence: task instruction in chat
```

## Qualification outcome and recovery

Report: `/home/mole/oteryn-cache/portalrs-e2e-report.md`; reproducible scripts and sanitized evidence: `/home/mole/oteryn-cache/portalrs-e2e/`. All identity browser paths pass in both locales. Disabled account and public read surfaces still return their intentional 503s; no flags or gates were weakened. Registration mail delivery and stale registration-session verification failures are fixed with regression tests. The existing rate-limit rollover test now observes PostgreSQL's actual minute boundary; quota and independence assertions are unchanged. Initial unbounded DB lock-memory exhaustion and the reproduced timer failure are preserved as failed attempts in the report. Final bounded workspace run is green.

Gateway inspection was read-only and environment names only. A second gateway can share the isolated portal network namespace and reach its loopback admission listener; successful admission remains unavailable until its owning identity and admission adapters replace `Unavailable`. No second gateway was started.

Portal, worker, Mailpit, original portal DB/network/data volume and final image are intentionally retained for the requester. Disposable validation container and intermediate image were removed. Anonymous checks volume `b70f3b1f2c32157f8c01d3bdaedb2cee7c5e33f2194ed9877eb65d1d2a2b1efe` was implicitly created by PostgreSQL; its provenance is retained in `checks-volume-provenance.json`. The user's prefix-only resource rule prevents its deletion without an exact exception. Approval or explicit retention was requested asynchronously and is pending. No out-of-prefix removal was attempted. The reproduction helper now explicitly mounts a prefix-named test volume.

The checkpoint head is captured before the final test/evidence commit; the external report records all final commit SHAs. No GitHub write, push, PR, integration or programme completion is authorized or claimed. Testing and report delivery are complete; resource closeout remains waiting for the exact volume decision.
