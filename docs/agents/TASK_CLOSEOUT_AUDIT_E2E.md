# Task Closeout, PR Hygiene, Audit and E2E Contract

```yaml
task_closeout_policy_version: 3
remediation_audit_specialization: docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
```

## Purpose

Implementation completion is not task completion. Terminal status requires complete acceptance, self-review, applicable audit, real applicable E2E, exact-head CI, PR hygiene, task archival and ownership release.

For remediation tasks, `REMEDIATION_AUDIT_RISK_GATE.md` controls whether a distinct independent auditor is required. Every remediation still requires exact-head self-review.

## Lifecycle

```yaml
task_lifecycle:
  - implementing
  - validating
  - self_reviewing
  - audit_gate
  - auditing_when_required
  - e2e_testing
  - final_ci
  - closing_prs
  - ready_to_archive
  - completed
```

Do not move directly from implementation to completed.

## Closeout sequence

```text
implementation
→ focused validation
→ component/integration validation
→ exact-head self-review
→ audit-risk decision
→ independent audit only when REQUIRED or requested OPTIONAL
→ finding remediation by the implementation owner
→ real E2E or justified NOT_APPLICABLE
→ final exact-head CI
→ PR/review cleanup
→ merge/terminal delivery
→ outcome verification
→ Issue/task archive
→ ownership/lease release
```

If the target head changes, rerun every affected downstream gate.

## One remediation owner

One remediation Issue normally has one implementation owner. That owner remains responsible through PR maintenance, finding remediation, merge, Issue closure, task archival and release.

An independent auditor never takes Issue ownership. Findings return to the same owner. A different implementation session may resume only through valid continuation/takeover of the same owner role.

## Related PR inventory

Search by task ID, Issue, programme, branch, changed contracts, producer/consumer dependencies, validation, audit and superseded attempts.

Every related PR must have one terminal classification:

```yaml
pr_terminal_state:
  - merged
  - closed_superseded
  - closed_duplicate
  - closed_obsolete
  - closed_invalid
  - closed_request_only
```

An unintentionally open PR, abandoned draft, unresolved requested change or superseded attempt blocks completion.

For every related PR verify repository, base, head, exact SHA, changed-file set, required CI, unresolved review threads and final merge/close evidence.

PASS-only audits, self-review, CI evidence, E2E evidence, Issue comments and ordinary ownership release do not require separate PRs.

## Self-review

Every implementation owner records:

```yaml
self_review:
  result: PASS | FAIL
  exact_head: <sha>
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true | NOT_APPLICABLE
  authorization_and_data_exposure_checked: true
  compatibility_and_rollback_checked: true
  related_prs_checked: true
  findings: []
  evidence: []
```

Self-review findings are fixed before readiness. Self-review must never be represented as independent audit.

## Remediation audit gate

Record the current decision:

```yaml
audit_gate:
  version: 1
  requirement: NOT_REQUIRED | OPTIONAL | REQUIRED
  mandatory_triggers: []
  optional_triggers: []
  unknown_or_conflict: []
  rationale: <evidence>
  independent_audit:
    result: NOT_REQUIRED | NOT_REQUESTED | PENDING | PASS | FAILED
    generation: <integer>
    evidence: []
```

Rules:

- `NOT_REQUIRED` is allowed only when every mandatory trigger is disproved and no material uncertainty remains.
- `OPTIONAL` may be requested; otherwise record `NOT_REQUESTED` with rationale.
- `REQUIRED` needs an unchanged-target independent PASS before merge.
- The implementation owner cannot waive a mandatory trigger.
- A material scope/risk/head change recomputes the gate.

## Independent audit when applicable

A valid independent auditor:

- has fresh independent context;
- reads trusted acceptance directly;
- inspects exact PR/base/head and primary evidence;
- attempts to falsify completion;
- does not mutate or remediate the target;
- records exact whole-diff and Issue verdicts;
- returns findings to the implementation owner.

An auditor who writes a target fix loses final-auditor eligibility for that generation.

```yaml
independent_audit:
  generation: <integer>
  auditor: <identity/session>
  target:
    pull_request: <number>
    base_sha: <sha>
    head_sha: <sha>
  whole_diff: PASS | FAILED | PENDING
  issue_verdict: PASS | FINDING | PENDING
  material_findings_open: <integer>
  target_unchanged: true
```

Critical, high and material-medium findings block completion.

## Audit matrix

Inspect applicable:

```yaml
audit_matrix:
  acceptance_criteria: required
  scope_and_vertical_slice: required
  backend: when_applicable
  frontend_or_client: when_applicable
  persistence_and_migrations: when_applicable
  api_or_protocol_contract: when_applicable
  authorization_and_validation: when_applicable
  failure_and_recovery_states: when_applicable
  localization_accessibility_responsive_ui: when_applicable
  security_and_secret_boundaries: required
  compatibility_and_rollout: when_applicable
  concurrency_idempotency_rollback: when_applicable
  logging_and_data_exposure: required
  test_coverage_and_evidence: required
  documentation_and_operability: when_applicable
  related_pr_and_task_hygiene: required
```

For `NOT_REQUIRED`, the implementation owner's self-review covers applicable matrix items and records why a distinct auditor is unnecessary.

## Finding remediation

For a material finding:

1. return to implementing;
2. preserve stable finding ID/evidence;
3. repair the smallest complete scope by the same implementation owner;
4. rerun focused and affected integration checks;
5. rerun E2E when behavior may change;
6. update exact-head evidence;
7. if independent audit remains required, create a new audit generation.

Do not archive with unresolved material findings.

## End-to-end validation

For user-facing work prove the real actor, real frontend/client, real backend/system contract, authorization, valid/invalid behavior, persistence, visible outcome and recovery states.

A backend API test does not replace frontend E2E. A mocked frontend does not replace integration E2E.

For non-UI work test:

```text
real input → public/system entry → processing → persistence/external effect → observable output
```

When required E2E cannot run:

```yaml
e2e:
  result: NOT_RUN
  blocker: <exact blocker>
  attempted: <actions>
  required_environment: <requirement>
  next_action: <one action>
```

Use `NOT_APPLICABLE` only with a concrete reason.

## Final exact-head CI

Required checks must pass on the exact final head. A head change invalidates affected CI, E2E and required audit evidence.

Do not rerun unchanged failures without diagnosis and do not weaken checks to obtain green status.

## Terminal evidence

```yaml
closeout:
  implementation_owner: <claim/session>
  implementation_complete: true
  self_review:
    result: PASS
    exact_head: <sha>
  audit_gate:
    requirement: NOT_REQUIRED | OPTIONAL | REQUIRED
    result: NOT_REQUIRED | NOT_REQUESTED | PASS
  e2e:
    result: PASS | NOT_APPLICABLE
    reason: <when NOT_APPLICABLE>
  final_ci:
    head: <sha>
    result: PASS
  related_prs:
    open: 0
    unresolved_threads: 0
    terminal: []
  issue_terminal: true
  task_status: completed
  task_archived: true
  ownership_released: true
```

Do not complete when self-review failed, a mandatory audit is absent, requested optional audit failed/pends, E2E is incomplete, exact-head CI is red, related PRs/review threads remain, Issue/task state is inaccurate or ownership remains claimed.

## Parallel remediation

Parallel commands create several independent Issue owners. No auditor is permanently reserved. Allocate an AUDIT ONLY role only when a valid REQUIRED/requested OPTIONAL handoff exists.

Workers do not wait for peers or internal roles. They persist durable state and use `ROTATE` at a required role boundary.

## Archival

Pre-merge task records may use `completed_on_merge` bound to exact PR/head/merged=true. Closing without merge cannot leave the task completed or ownership released.

Unavoidable post-merge governance-only reconciliation follows lifecycle batching; do not create one archive PR per repair.
