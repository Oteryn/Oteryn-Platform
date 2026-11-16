---
task_id: OTERYN-20261115-a3-08-store-wallet
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 3 store and wallet repairs

Governing issue: https://github.com/Oteryn/Oteryn-Platform/issues/536 (OPEN,
verified 2026-11-15). Owner requests SW3-02, SW3-05 and SW3-06 from base
90234d8d2d0a51a654b8ff6947d38372a740acba on branch
claude/happy-davinci-ijz3wy-fix-a3-08-store-wallet. Single agent; no PR or merge.

Owned paths: portal-rs/bin/oteryn-portal, portal-rs/crates/wallet and
portal-rs/crates/store. Parallel RBAC task's excluded files remain untouched.
No remote task branch existed at admission. All three findings confirmed.

Implementation and required validation complete; ready for coordinator review.
Source branch retained for coordinator review. No production or Game access.

## Confirmed behavior and changes

- SW3-02: wire cancellation previously called the active-only coin-account
  preflight. It now admits safe returns to suspended holders, preserving missing
  identity and reservation ownership refusals. The production router regression
  verifies release, unchanged receipt on exact replay and final ledger totals.
- SW3-05: coin exports previously preceded the public clone. A tested composition
  helper now captures public/native routes first and adds all seven coin exports
  only internally. Both flag states retain internal 401/503 guards. Real binary HTTP and native
  TLS tests also require 404 for all seven paths, including spoofed identities.
- SW3-06: catalog import and activation previously authorized without the RBAC
  fence. Their shared authorization path locks the actor first, then holds shared
  advisory fence 71010 through catalog commit. The production permission writer
  uses the exclusive fence. The regression blocks activation on the pointer and
  proves real SetPermissions cannot commit ahead of activation.

All three regressions failed before the corresponding fixes. Baseline failures:
409 account_inactive, public 503 instead of 404, and permission revocation
committing ahead of activation. Local logs: /tmp/sw3-red-wallet.log and
/tmp/sw3-red-portal2.log. Focused green PostgreSQL durations (Rust 1.97):
`sw3_02_suspended_seller_cancels_through_coin_router` 2.261s;
`sw3_06_catalog_activation_fences_set_permissions` 1.763s.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T21:22:27Z
head: 90234d8d2d0a51a654b8ff6947d38372a740acba
branch: claude/happy-davinci-ijz3wy-fix-a3-08-store-wallet
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/bin/oteryn-portal
  - portal-rs/crates/wallet
  - portal-rs/crates/store
proven:
  - All three findings reproduced and focused regressions pass
  - Public/native coin paths are absent with flags enabled and disabled
  - Catalog authorization retains the shared RBAC fence through commit
  - No excluded RBAC paths or migrations changed
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs
  - portal-rs/bin/oteryn-portal/src/coin_listener_tests.rs
  - portal-rs/bin/oteryn-portal/tests/public_listener_guard.rs
  - portal-rs/crates/store/src/catalog.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wallet/src/wire.rs
  - portal-rs/crates/wallet/tests/coin_http/mod.rs
  - docs/agents/tasks/active/OTERYN-20261115-a3-08-store-wallet.md
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Rust 1.97 repository pin
  - command: cargo clippy --locked --all-targets -p oteryn-wallet -p oteryn-store -p oteryn-portal -- -D warnings
    result: PASS
    evidence: /tmp/sw3-clippy.log
  - command: cargo test --locked -p oteryn-wallet -p oteryn-store -p oteryn-portal
    result: PASS
    evidence: /tmp/sw3-tests-final.log; 344 passed, zero failed, two unchanged browser ignores; --test-threads=1 with real local PostgreSQL
blockers:
  - none
next_action: Coordinator reviews the authorized fix branch; no PR or merge requested
```


Validation recovery: the initial parallel portal suite exhausted the local
PostgreSQL shared lock table (SQLSTATE 53200; 19 fixture failures). A serial
rerun passed all 185 portal unit tests, then exposed the existing listener test's
old expectation of coin 401 responses. Those seven expectations now require
404, matching the repaired boundary. Added real HTTP/native TLS coverage for
both coin flag states and isolated simultaneous listener fixtures. Both listener
tests pass; portal Clippy was rerun and passed. Complete suites run serially to
avoid the local PostgreSQL lock-table limit. No tests were removed or ignored.


Final complete suites: 344 passed, zero failed, two pre-existing optional browser
ignores. Formatting and Clippy for all three touched crates passed; final portal
Clippy also passed after adding listener integration coverage. Final PostgreSQL
regression durations: `sw3_02_suspended_seller_cancels_through_coin_router` 1.246s;
`sw3_06_catalog_activation_fences_set_permissions` 1.369s. Final full test log:
/tmp/sw3-tests-final.log. All tests ran with the repository-pinned Rust 1.97.
