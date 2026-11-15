---
task_id: OTERYN-20261115-audit-a2-07-portal-bin
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Portal binary audit wave 2 repairs

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536 (OPEN, verified 2026-11-15). Owner supplied exact findings W2-01, W2-02, W2-03, base c6fdc5438a4afa91f32d547b7cfc61f22bf21139, branch and push instructions. Single agent; no PR or merge requested. Repository identity verified as Oteryn/Oteryn-Platform. Live open PR search found no competing happy-davinci branch. Existing active Bazaar packet owns unrelated paths. Jira synchronization remains with coordinator.

Owned paths: portal-rs/bin/oteryn-portal, crates/db/src/outbox.rs, crates/gameauth/src/devices_tls.rs, crates/store/src/purchase_accounts.rs. No migrations or contract changes. All money/game feature gates retained. Local PostgreSQL only.

Acceptance: bounded and supervised listener failure; deadlines and connection cancellation for schedule/claim; separate account locking before dependent eligibility reads; regression tests and required formatting, Clippy and touched-crate suites; requested commit and non-force branch push.

Status: ready for the requested branch publication after successful validation. Source branch disposition: retain for coordinator review; no PR requested.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T17:31:00Z
head: c6fdc5438a4afa91f32d547b7cfc61f22bf21139
branch: claude/happy-davinci-ijz3wy-fix-a2-07-portal-bin
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/bin/oteryn-portal/
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/gameauth/src/devices_tls.rs
  - portal-rs/crates/store/src/purchase_accounts.rs
proven:
  - All three findings confirmed on the requested base
  - W2-01 negative control with original silent accept policy fails after 5.01 seconds
  - W2-02 both base worker stages fail the external two-second bounds
  - W2-03 base settlement reproduces Paid instead of Processing after restriction commits
  - Store base adapter separately reproduces Active instead of Restricted after the same lock wait
  - Settlement regression covers restriction, suspension and typed ban and checks no wallet credit
  - Repository identity is Oteryn/Oteryn-Platform and governing issue 536 is open
  - No migrations, feature flags, contract files, workflow files or external game repositories changed
derived: []
unknown:
  - Coordinator Jira mapping and synchronization
conflicts: []
first_failure:
  marker: PostgreSQL parallel lock-memory exhaustion
  evidence: First broad run had six gameauth errors with PostgreSQL code 53200; serial execution selected without changing tests
rejected_hypotheses:
  - Pausing after only the first sandbox eligibility check targets settlement; Payments intent reads eligibility again, so the regression pauses after that second check
changed_paths:
  - portal-rs/bin/oteryn-portal/src/coin_purchases.rs
  - portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs
  - portal-rs/bin/oteryn-portal/src/internal_tls.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/gameauth/src/devices_tls.rs
  - portal-rs/crates/store/src/purchase_accounts.rs
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Final source formatted with installed Rust 1.97 pin
  - command: cargo clippy --locked --all-targets -p oteryn-db -p oteryn-gameauth -p oteryn-store -p oteryn-portal -- -D warnings
    result: PASS
    evidence: Final source on Rust 1.97; no tools installed
  - command: cargo nextest run --locked -p oteryn-db -p oteryn-gameauth -p oteryn-store -p oteryn-portal with focused regression filter and --test-threads 1
    result: PASS
    evidence: Five passed; accept retry 2.602s, supervisor 0.006s, real PostgreSQL settlement and Store matrix 1.330s, blocked claim 1.400s, conflicting schedule 1.274s
  - command: cargo test --locked -p oteryn-db -p oteryn-gameauth -p oteryn-store -p oteryn-portal -- --test-threads=1
    result: PASS
    evidence: 433 passed, zero failed, two existing browser ignores; no tests or ignore attributes weakened or removed
  - command: cargo test --locked -p oteryn-db -p oteryn-gameauth -p oteryn-store -p oteryn-portal -- --ignored --test-threads=1
    result: PASS
    evidence: Both existing ignored PostgreSQL and Chromium browser tests explicitly executed and passed in 4.77s and 4.96s; all 435 unique tests executed across the two runs
  - command: prepared browser dependency completion
    result: PASS
    evidence: Reused native Node, Chromium and prepared playwright-core; installed only missing axe packages in /tmp/a207-browser-modules without reinstalling environment tools or changing repository dependencies
blockers: []
next_action: Commit with the specified author and trailers, then push the owner-specified branch for coordinator review
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested this fix branch for coordinator review, without a PR or merge
source_branch_evidence: All exact-source formatting, Clippy, regression, serial suite and browser checks pass; retain the requested branch for coordinator review
```


Validation logs are local under /tmp/a207-*.log; no credentials or database URLs are recorded in this packet. The first parallel run's six PostgreSQL shared lock-memory errors were resolved by serial execution, without product or test changes. Negative controls restored the old accept policy at the injection seam and the exact base worker/account adapters; all fixed sources were restored before final validation. The second settlement negative control failed specifically with Paid versus Processing; the first independently caught the Store Active versus Restricted snapshot race.

The checkpoint head records the admission base; this packet and the product changes are published together in the owner-requested commit. Its Git identity supplies the immutable candidate SHA. No PR or merge is requested or performed. The coordinator owns subsequent review and Jira synchronization.
