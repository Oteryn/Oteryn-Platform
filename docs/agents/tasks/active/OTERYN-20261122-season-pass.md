---
task_id: OTERYN-20261122-season-pass
governing_issue: 2002
required_reads: []
search_first: []
optional_reads: []
---

# F22c Season pass adapters

Governing issue [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002).

## Acceptance criteria

- Enable the Platform account and Wallet adapters through explicit default-off testing flags.
- Prove purchase replay, conflicting operation reuse, insufficient funds and ledger reconciliation.
- Verify browser behavior in Polish and English at 1440 and 390 pixels.
- Keep Game progress and cosmetic reward issuance closed pending applicable authority.

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/minigames/src/pass/
  - portal-rs/e2e/specs/season-pass.spec.mjs
modules:
  - season pass
dependencies:
  - Wallet
  - G-11 private journal
  - Store cosmetic authority
blockers:
  - Season-pass journal evidence mapping and accepted Store cosmetic authority.
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T16:00:56Z
head: 2395e2438ca4a22de9da8ba09fdda65f5bbc4a96
branch: codex/feat-season-pass
pr: none
status: waiting
context_routes: []
owned_paths:
  - portal-rs/crates/minigames/src/pass/
  - portal-rs/e2e/specs/season-pass.spec.mjs
proven:
  - The claim operation key serializes requests across accounts and seasons. An exact replay returns the recorded receipt and a changed binding conflicts before enqueue.
  - Signed-in pass overview reads return zero progress for a new player without creating a row.
  - The Store purchase delivery and wire contracts are proposed.
  - G-9 revision 3 and G-11 revision 4 accept the journal wires. They do not establish season-pass reward evidence.
derived:
  - Platform testing can use its own account eligibility and Wallet ledger without a Game effect.
unknown:
  - Accepted mapping for Game evidence to season-pass challenges.
conflicts: []
first_failure:
  marker: wallet replay HTTP status
  evidence: The pass payment adapter treated Wallet idempotency conflicts as access refusals.
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/minigames/src/pass/
  - portal-rs/e2e/
  - portal-rs/docs/FLAG_BOUNDARY.md
validation:
  - command: cargo test --locked -p oteryn-minigames --lib pass::
    result: PASS
    evidence: All 12 pre-review focused tests passed against PostgreSQL 18.6.
  - command: RUST_TEST_THREADS=1 cargo test --locked -p oteryn-minigames --lib
    result: PASS
    evidence: All 83 library tests passed against PostgreSQL 18.6 after the review fixes.
  - command: Playwright season-pass.spec.mjs
    result: PASS
    evidence: pl-1440, pl-390, en-1440 and en-390 passed against the native local browser launcher.
  - command: Cross-repository harness seed CLI smoke
    result: PASS
    evidence: The isolated public, internal mTLS and native admission listeners became ready and the verified account seed succeeded.
  - command: cargo fmt --all --check
    result: PASS
    evidence: Workspace Rust formatting check passed.
blockers:
  - Store cosmetic authority and a verified journal challenge mapping remain open.
next_action: Obtain the accepted Game progress mapping and Store cosmetic authority before enabling their adapters.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: The requested deliverable is a dedicated pushed branch.
source_branch_evidence: No PR or merge is authorized for this task.
```
