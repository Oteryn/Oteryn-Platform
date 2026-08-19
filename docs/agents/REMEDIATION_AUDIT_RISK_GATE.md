# Remediation Self-Review and Risk Gate

```yaml
remediation_risk_gate_version: 2
legacy_path_name: REMEDIATION_AUDIT_RISK_GATE.md
repository: Oteryn/Oteryn-Platform
one_issue_one_owner: true
external_repair_auditor_required: false
```

This filename is retained for compatibility with existing links and prompts. The controlling policy no longer routes repaired Issues to a different agent for approval.

## Core lifecycle

```text
one Issue
→ one implementation owner
→ one deterministic branch
→ one delivery PR
→ implementation
→ exact-head self-review
→ focused and risk-proportional validation
→ required CI and applicable E2E
→ merge
→ Issue closure, archival and ownership release
```

The implementation owner remains responsible from claim through terminal closeout. Findings from self-review, CI, normal review, or a later platform-wide audit return to that same owner while the Issue remains active.

A separate continuous-audit programme may inspect the platform and create new findings. It is independent discovery, not a mandatory per-repair merge handoff.

## Validation states

Every repair records exactly one current state:

```yaml
validation_gate:
  version: 2
  intensity: STANDARD | HEIGHTENED | BLOCKED
  classified_by: <implementation owner>
  classified_at: <ISO-8601 timestamp>
  risk: critical | high | medium | low
  triggers: []
  unknown_or_conflict: []
  rationale: <bounded evidence-based explanation>
  self_review:
    result: PENDING | PASS | FAIL
    exact_head: <sha or none>
    evidence: []
```

Recompute the gate after a material scope, risk, dependency, rollout, rollback, compatibility or head change.

## STANDARD

`STANDARD` is allowed when all are proven:

- risk is low or bounded medium;
- scope is one coherent, reversible Issue;
- acceptance is deterministic;
- focused validation covers the affected behavior;
- applicable E2E is `PASS` or validly `NOT_APPLICABLE`;
- required exact-head CI passes;
- no material finding, `UNKNOWN`, `CONFLICT`, requested change or unresolved review thread remains.

## HEIGHTENED

Use `HEIGHTENED` for critical/high risk or material changes involving:

- authentication, authorization, sessions, MFA, credentials, secrets, recovery, admin or RBAC;
- payments, balances, currency, entitlements or economically sensitive state;
- data integrity, duplication, concurrency, locking, idempotency, race conditions or transactions;
- migrations, schema authority, destructive data change, shared tables or uncertain rollback;
- public API, protocol, generated contracts/types, compatibility, downgrade or version negotiation;
- dependencies, CI, branch protection, deployment, infrastructure, production configuration, protected environments or rollback semantics;
- durable architecture, new subsystems, data ownership, public boundaries or cross-module dependency direction;
- atomic cross-repository compatibility or rollout ordering;
- broad vertical slices whose blast radius requires wider deterministic evidence.

`HEIGHTENED` requires stronger focused regression tests, negative-path evidence, rollback/compatibility checks and the full applicable final validation suite. It does not create a mandatory different-agent audit.

## BLOCKED

Use `BLOCKED` when required authority, rollback, compatibility, environment, evidence, ownership or a material fact is unresolved. `UNKNOWN` and `CONFLICT` never silently become `STANDARD` or `HEIGHTENED PASS`.

## Required self-review

Every repair records:

```yaml
self_review:
  result: PASS | FAIL
  exact_head: <sha>
  acceptance_checked: true
  full_diff_checked: true
  related_prs_checked: true
  negative_paths_checked: true | NOT_APPLICABLE
  rollback_checked: true
  compatibility_checked: true | NOT_APPLICABLE
  findings: []
  evidence: []
```

Self-review must cover the whole exact-head diff, acceptance criteria, negative paths, rollback, compatibility, related PRs and all current findings. The owner repairs findings and repeats relevant focused validation before readiness.

Self-review must not be represented as an external or independent audit.

## Parallel workers

A command requesting `N` repair agents means up to `N` distinct end-to-end implementation owners, each claiming one different Issue. No slot is reserved for an auditor or integrator. A losing claim selects another safe Issue rather than waiting.

## Repair trains

Ordinary product/runtime remediation remains one Issue, one owner and one PR. Do not combine unrelated repairs or wait to fill a train. Lifecycle-only reconciliation may still be batched when it is homogeneous, reversible and preserves per-Issue provenance.

## Merge gate

Merge requires:

- self-review `PASS` on the exact final head;
- risk-proportional focused tests;
- applicable E2E complete;
- required exact-head CI `PASS`;
- bounded rollback and proven compatibility where applicable;
- zero material findings, requested changes, unresolved threads, ownership conflicts, `UNKNOWN`, `CONFLICT` or blockers;
- normal branch protection and repository safety rules satisfied.

Do not create an audit-only Issue, frozen audit generation, audit PR or ownership transfer merely to approve a completed repair.

## Metrics

```yaml
repair_targets:
  one_issue_one_owner: 100_percent
  repairs_with_exact_head_self_review: 100_percent
  mandatory_external_repair_audits: 0
  self_reviews_mislabeled_independent: 0
  workers_waiting_for_internal_roles: 0
  ordinary_product_repairs_in_trains: 0
```
