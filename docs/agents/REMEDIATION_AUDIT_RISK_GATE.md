# Remediation Audit Risk Gate

```yaml
remediation_audit_risk_gate_version: 1
repository: blakinio/Oteryn-Platform
applies_to:
  - OTERYN_PLATFORM_REMEDIATION
controlling_specialization_over:
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/REPAIR_PR_ECONOMY.md
scope_of_specialization: whether a remediation delivery requires a distinct independent auditor
objective: keep one implementation owner responsible for one Issue end to end while requiring independent audit whenever material risk or uncertainty justifies it
```

## Core ownership rule

The default remediation lifecycle is:

```text
one Issue
→ one implementation owner
→ one deterministic branch
→ one delivery PR
→ implementation and self-review
→ applicable E2E and exact-head CI
→ merge
→ Issue closure, task archival and ownership release by the same owner
```

The implementation owner keeps responsibility from claim through terminal closeout. Creating an audit handoff does not transfer Issue ownership. If an independent auditor returns findings, the same implementation owner repairs them and produces the next candidate generation.

Every repair requires self-review. Independent audit is an additional risk control, not a substitute for implementation ownership, tests, E2E, CI, review-thread cleanup or exact-head verification.

## Audit decision states

Every remediation task and delivery PR must record exactly one current decision:

```yaml
audit_gate:
  version: 1
  requirement: NOT_REQUIRED | OPTIONAL | REQUIRED
  classified_by: <implementation owner or coordinator>
  classified_at: <ISO-8601 timestamp>
  risk: critical | high | medium | low
  mandatory_triggers: []
  optional_triggers: []
  disproved_triggers: []
  unknown_or_conflict: []
  rationale: <bounded evidence-based explanation>
  self_review:
    result: PENDING | PASS | FAIL
    evidence: []
  independent_audit:
    result: NOT_REQUIRED | NOT_REQUESTED | PENDING | PASS | FAILED
    generation: 0
    evidence: []
```

The decision must be made before the final candidate is declared ready and must be recomputed after a material scope, risk, dependency, rollout or head change.

`UNKNOWN` or `CONFLICT` affecting a mandatory trigger resolves to `REQUIRED` or blocks the task. It never resolves to `NOT_REQUIRED`.

## Mandatory independent-audit triggers

Set `requirement: REQUIRED` when any trigger is present:

1. Issue risk is `critical` or `high`, including `risk:critical` or `risk:high` labels.
2. Authentication, authorization, sessions, MFA, credentials, secrets, password recovery, protected data, admin/RBAC or security-policy boundaries change.
3. Payments, balances, currency, financial mutation, entitlements or economically sensitive state change.
4. Data integrity, duplication, concurrency, locking, idempotency, race conditions or transaction boundaries change materially.
5. A migration owns schema authority, removes or rewrites data, changes a shared table, is destructive, irreversible or has uncertain rollback.
6. Public API, protocol, generated contract/type, compatibility, downgrade or version-negotiation authority changes.
7. CI workflow, branch protection, deployment, infrastructure, production configuration, protected environment, release or rollback semantics change.
8. A durable architecture decision, new subsystem, data owner, public boundary or cross-module dependency direction changes.
9. An atomic cross-repository contract, required rollout order or external producer/consumer compatibility is involved.
10. The change is a large or broad vertical slice whose blast radius cannot be disproved by focused deterministic validation.
11. A reviewer, existing auditor, CI finding, accepted security policy or the owner explicitly requires independent audit.
12. The implementation owner records material uncertainty, conflicting evidence or cannot independently disprove a relevant high-impact failure mode.
13. The task changes audit, authorization, merge, branch-protection or closeout policy itself.

An implementation owner cannot downgrade or waive a mandatory trigger. Removing a trigger requires changing the underlying scope/evidence, not editing the classification label.

## Optional independent audit

Set `requirement: OPTIONAL` when no mandatory trigger exists but one or more of these applies:

- medium-risk behavior crosses several internal layers but remains reversible and well tested;
- a complex algorithm, state machine or recovery path would benefit from adversarial review;
- test coverage is adequate but the failure mode is difficult to reproduce;
- the implementation owner requests a second perspective;
- review comments indicate non-material uncertainty that does not yet meet a mandatory trigger.

For `OPTIONAL`, the implementation owner or coordinator may request audit. If no audit is requested, record `independent_audit.result: NOT_REQUESTED` and explain why self-review plus validation is sufficient.

## Independent audit not required

`requirement: NOT_REQUIRED` is allowed only when all are proven:

- risk is `low` or bounded `medium`;
- no mandatory trigger applies;
- scope is one coherent Issue and normally one module or narrowly coupled path set;
- the change is reversible and has a bounded rollback;
- no destructive migration, security authority, public contract authority, protected rollout or cross-repository atomicity is involved;
- acceptance is deterministic and supported by focused/component validation;
- required E2E is PASS or validly `NOT_APPLICABLE` with a concrete reason;
- required exact-head CI passes;
- no material review finding, `UNKNOWN` or `CONFLICT` remains.

A documentation typo, bounded copy correction, isolated test-fixture repair, narrow low-risk UI defect or similarly reversible change may qualify. Classification is evidence-based; file type alone never proves low risk.

## Required self-review

The implementation owner always performs and records self-review before merge:

```yaml
self_review:
  result: PASS | FAIL
  exact_head: <sha>
  acceptance_checked: true
  full_diff_checked: true
  related_prs_checked: true
  negative_paths_checked: <true or NOT_APPLICABLE with reason>
  rollback_checked: true
  findings:
    - <finding or none>
  evidence:
    - <test, path, run or behavior>
```

Self-review may repair findings on the same branch. It must never be called `independent_audit`, `independent PASS` or `fresh validator`.

## Required-audit handoff

When `requirement: REQUIRED`, the implementation owner:

1. finishes coherent implementation and self-review;
2. runs focused/component validation and applicable E2E preparation;
3. freezes the exact candidate target;
4. publishes an exact audit handoff;
5. keeps Issue, branch, task and PR ownership;
6. sets checkpoint `ready` with one audit `next_action`;
7. returns `ROTATE` when no eligible distinct auditor can run in the same invocation.

The auditor is read-only toward the target, records exact whole-diff and per-Issue verdicts, and returns findings to the same implementation owner. A changed target invalidates the previous audit generation.

After PASS, the implementation owner resumes and completes final CI verification, merge, Issue closure, archival and ownership release.

## Parallel workers

A command requesting `N` repair agents means up to `N` end-to-end implementation owners, each claiming one distinct Issue. No audit slot is permanently reserved.

The coordinator allocates an audit role only when a valid `REQUIRED` handoff exists. An available agent may switch to `AUDIT ONLY` only when it is independent from the target implementation and has not written or remediated target commits.

Workers never wait for peers or a future audit slot. They persist durable state and rotate at a real role boundary.

## Repair trains

Ordinary product/runtime remediation does not use repair trains by default. It remains one Issue, one owner and one PR.

A repair train is exceptional and requires explicit coordinator authorization. It is eligible only for homogeneous, low-risk, independently reversible mechanical, documentation, test-fixture or governance changes that:

- have no mandatory audit trigger other than a policy-change trigger created by the train itself;
- do not touch authentication, payments, migrations/schema authority, public protocol/API authority, dependencies/lockfiles, CI/deployment, production or durable architecture boundaries;
- preserve exact per-Issue provenance and rollback;
- do not cause a coherent repair to wait for another candidate.

Lifecycle-only reconciliation remains governed by `LIFECYCLE_CLOSEOUT_BATCHING.md` and is preferred over an active repair train after implementation is terminal.

## Merge gate

For `NOT_REQUIRED` or unrequested `OPTIONAL` audit:

- self-review PASS;
- audit-gate evidence complete;
- applicable E2E complete;
- required exact-head CI PASS;
- review threads and related PRs terminal;
- no mandatory trigger, material finding, `UNKNOWN` or `CONFLICT` remains.

For `REQUIRED` audit, all of the above apply plus a valid unchanged-target independent PASS.

## Prohibited patterns

Do not:

- require a separate auditor for every trivial repair;
- let an implementation owner waive a mandatory trigger;
- claim self-review is independent audit;
- transfer Issue ownership to the auditor;
- let an auditor repair the target and still issue final PASS for that generation;
- reserve an idle audit slot when no required handoff exists;
- use `WAITING` for internal role rotation;
- combine unrelated product repairs merely to reduce PR count;
- classify a task `NOT_REQUIRED` because only documentation or tests changed without checking semantic risk;
- merge with incomplete audit-gate evidence.

## Metrics

```yaml
selective_audit_targets:
  repairs_with_documented_self_review: 100_percent
  mandatory_triggers_without_independent_pass: 0
  self_reviews_mislabeled_independent: 0
  low_risk_repairs_forced_to_external_audit: 0
  implementation_ownership_transfers_to_auditor: 0
  idle_reserved_audit_slots: 0
  workers_waiting_for_internal_roles: 0
  ordinary_product_repairs_in_trains: 0
```
