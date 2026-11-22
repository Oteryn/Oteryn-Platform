---
task_id: OTERYN-20261122-gateway-badinput
governing_issue: 2002
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - portal-rs/e2e/README.md
---

# OTERYN-20261122-gateway-badinput

Issue [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002) tracks refusal coverage in the native login gateway and the private admission listener. The assigned branch is `codex/e2e-gateway-badinput`.

- Compare malformed, truncated, oversized, invalid UTF-8 and unsupported-version input against the documented refusal bytes and existing vectors.
- Verify bounded incomplete-body handling over the gateway listener.
- Check the bearer and mTLS listener boundaries and concurrent refusals of unusable tickets with the portal harness.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T16:00:00Z
head: 4f8f2feb3ac9e689bb67ccb040c9d43f6cec48dc
branch: codex/e2e-gateway-badinput
pr: 2012
status: completed
context_routes:
  - native gateway and private admission
owned_paths:
  - portal-rs/crates/gateway/tests/bad_input.rs
  - portal-rs/crates/gateway/tests/deadlines.rs
  - portal-rs/e2e/harness.mjs
  - portal-rs/e2e/native-admission-refusals.mjs
  - portal-rs/e2e/README.md
proven:
  - The admission listener uses a loopback bearer credential. Game mTLS is on a separate listener.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/gateway/tests/bad_input.rs
  - portal-rs/crates/gateway/tests/deadlines.rs
  - portal-rs/e2e/harness.mjs
  - portal-rs/e2e/native-admission-refusals.mjs
  - portal-rs/e2e/README.md
validation:
  - command: cargo test --locked -p oteryn-gateway
    result: PASS
    evidence: All 53 baseline tests passed.
  - command: cargo test --locked -p oteryn-gateway --test bad_input --test deadlines
    result: PASS
    evidence: All 5 focused candidate tests passed.
  - command: cargo test --locked -p oteryn-gameauth admission::tests::
    result: PASS
    evidence: All 17 tests passed with PostgreSQL 18.6, including one committed admission for duplicate requests.
  - command: node portal-rs/e2e/native-admission-refusals.mjs <harness-report>
    result: PASS
    evidence: The real gateway and admission roles refused unusable tickets and preserved their database state. The harness also verified both listener authentication boundaries.
blockers:
  - none
next_action: none; delivered by PR #2012 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: deleted
source_branch_reason: Delivered by squash merge of PR #2012.
source_branch_evidence: PR #2012 merged as 4f8f2feb3ac9e689bb67ccb040c9d43f6cec48dc on 2026-11-22; refs/heads/codex/e2e-gateway-badinput no longer exists on origin
```
