---
task_id: OTERYN-20261115-a3-10-characters
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 3: characters

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/536 (open Audit & Repair programme). Owner supplied the exact base, branch, findings, path exclusions and publication instructions. No PR requested.

## Goal and ownership

Fix CW3-01 recovery credit reuse and CW3-02 bootstrap permission serialization. Own `portal-rs/crates/characters/`; extend `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs` to exercise the real RBAC Change API without a circular crate dependency. Do not modify parallel RBAC-owned files. No schema or contract changes.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T21:08:17Z
head: 90234d8d2d0a51a654b8ff6947d38372a740acba
branch: claude/happy-davinci-ijz3wy-fix-a3-10-characters
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/characters/
  - portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs
proven:
  - Both findings confirmed against the specified integration base.
  - CW3-01 regression failed before the receipt fix (1.17 seconds).
  - CW3-02 regression failed before the RBAC fence fix (0.97 seconds).
derived: []
unknown:
  - Coordinator integration and branch deletion are outside this worker task.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/characters/src/bootstrap/issue.rs
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/characters/src/services/README.md
  - portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs
  - docs/agents/tasks/active/OTERYN-20261115-a3-10-characters.md
validation:
  - command: regressions before fixes
    result: PASS
    evidence: Both failed on the intended invariant assertions, not setup errors.
  - command: cargo fmt --all --check
    result: PASS
    evidence: Rust 1.97.0; /tmp/cw3-fmt.log
  - command: cargo clippy --locked --all-targets -p oteryn-characters -p oteryn-portal -- -D warnings
    result: PASS
    evidence: Rust 1.97.0; /tmp/cw3-clippy.log
  - command: cargo test --locked -p oteryn-characters -p oteryn-portal -- --nocapture --test-threads=1
    result: PASS
    evidence: 280 passed, 0 failed; one pre-existing Chromium checkout test ignored by the standard suite. Character unit tests 92/92 (26.95s), portal unit tests 184 passed (244.94s), four portal integration tests passed. No missing-database skips. Rust 1.97.0; /tmp/cw3-tests.log. CW3-01 1.573996147s; CW3-02 1.187854649s.
blockers:
  - none
next_action: Publish this reviewed candidate on the owner-specified branch for coordinator integration.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requests push without PR; coordinator owns integration and eventual deletion.
source_branch_evidence: Current task instructions.
```

## Delivery evidence

CW3-01: every newly reconciled committed replacement locks and validates its originating credit, reclaims an unused claim, and refuses another consumer or invalid origin. The regression covers rejection, competing consumption, restore, invalid-origin refusal, commit, replay, acknowledgement and resync, then refuses another free rename.

CW3-02: issuance prepares actor/subject account locks in UUID order, takes shared RBAC fence 71010, then checks permission; retains all locks through issuance and audit. The CLI regression observes actual PostgreSQL authority/fence waits around the real `Change::SetPermissions` API and verifies subsequent issuance is denied after revocation.

No schema, contract, feature-flag or excluded RBAC-path changes. Browser E2E is not applicable to these receipt/CLI changes; the real PostgreSQL receipt/restore and CLI/RBAC paths are exercised. Publication is an ordinary push only; no PR, merge or deployment is authorized. The checkpoint head records the admission base; the final commit is resolved from the named branch and reported to the owner.
