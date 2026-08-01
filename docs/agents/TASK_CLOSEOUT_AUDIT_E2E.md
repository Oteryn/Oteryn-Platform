# Task Closeout, PR Hygiene, Audit and E2E Contract

## Purpose

Implementation completion is not task completion. A task reaches terminal status only after the delivered outcome is independently audited, exercised end to end, validated on the exact final head, and cleaned up across task records, ownership, branches, reviews, and related pull requests.

Repository merge, production, authorization, and safety rules remain authoritative when stricter.

## Required lifecycle

Use distinct states or equivalent repository phases:

```yaml
task_lifecycle:
  - implementing
  - validating
  - auditing
  - e2e_testing
  - final_ci
  - closing_prs
  - ready_to_archive
  - completed
```

Do not move directly from implementation to completed.

## Required closeout sequence

```text
implementation
→ focused validation
→ component/integration validation
→ fresh post-implementation audit
→ audit remediation
→ real E2E
→ final exact-head CI
→ PR and review cleanup
→ task archive or terminal status
→ ownership/lease release
→ programme barrier review
→ next READY task
```

If remediation changes the final head, rerun every affected downstream gate.

## Related PR inventory

Before closeout search by task ID, programme/wave, branch, changed contracts, producer/consumer dependencies, validation work, audit work, archive work, and superseded attempts.

Every related PR must have one intentional terminal classification:

```yaml
pr_terminal_state:
  - merged
  - closed_superseded
  - closed_duplicate
  - closed_obsolete
  - closed_invalid
  - closed_request_only
```

An unintentionally open PR, abandoned draft, unresolved validation PR, or superseded implementation attempt blocks task completion. If a required PR must remain open for an external dependency, the task remains `WAITING` or `BLOCKED`.

## PR closeout procedure

For every related PR:

1. verify repository, base, head branch and exact head SHA;
2. inspect the complete changed-file set and full relevant diff;
3. verify required CI on the exact final head;
4. inspect unresolved review threads, requested changes and material comments;
5. remediate valid findings;
6. merge only when repository policy and task authorization permit;
7. close duplicate, obsolete, superseded, invalid, or request-only PRs with an accurate reason;
8. confirm the final terminal state from GitHub/environment state;
9. release or delete the source branch when repository policy permits;
10. record exact merge/close evidence and zero unresolved threads.

Opening a replacement PR does not close the previous one. Green CI does not make a PR terminal.

## PR evidence

The terminal record should include:

```yaml
related_prs:
  - repository: <owner/repo>
    number: <number>
    purpose: implementation | integration | validation | audit | archive | superseded_attempt
    final_head: <sha>
    terminal_state: <state>
    merge_or_close_evidence: <exact evidence>
    unresolved_threads: 0
```

Include failed and superseded attempts, not only the final successful PR.

## Fresh post-implementation audit

Material tasks require a fresh audit after coherent implementation and integration validation.

The audit should use independent context and must not trust the implementer summary. Its objective is to falsify completion against the original acceptance inventory and resulting environment.

```yaml
audit_policy:
  validator: fresh
  independent_context: true
  implementation_authorized: false
  objective: falsify_acceptance
  trust_worker_summary: false
  inspect_final_diff: true
  inspect_environment_outcome: true
```

A trivial documentation-only task may use a proportionate audit, but the audit phase must still inspect final paths, links, contradictions, lifecycle state, and PR hygiene.

## Audit matrix

Inspect all applicable areas:

```yaml
audit_matrix:
  acceptance_criteria: required
  scope_and_vertical_slice: required
  backend: when_applicable
  frontend_or_client: when_applicable
  persistence_and_migrations: when_applicable
  api_or_protocol_contract: when_applicable
  authorization_and_validation: when_applicable
  loading_empty_error_recovery_states: when_applicable
  localization_accessibility_responsive_ui: when_applicable
  security_and_secret_boundaries: required
  compatibility_and_rollout: when_applicable
  concurrency_idempotency_rollback: when_applicable
  logging_and_data_exposure: required
  test_coverage_and_evidence: required
  documentation_and_operability: when_applicable
  stale_code_todos_and_dead_paths: required
  related_pr_and_task_hygiene: required
```

## Audit findings

Every finding uses durable evidence:

```yaml
audit_finding:
  id: <stable ID>
  severity: critical | high | medium | low | informational
  confidence: high | medium | low
  evidence: <path, command, behavior or environment result>
  impact: <observable consequence>
  disposition: fixed | accepted_risk | false_positive | blocked
  verification: <exact recheck>
```

Critical, high, or material medium findings block completion. The implementing worker may not accept its own material risk merely to finish. Risk acceptance requires the authority defined by repository policy.

## Remediation loop

For a material finding:

1. return to implementing;
2. keep the finding ID and evidence;
3. repair the smallest complete scope;
4. run focused validation;
5. rerun affected integration checks;
6. rerun the failed audit check;
7. rerun E2E when user/system behaviour could be affected;
8. update the exact-head evidence.

Do not archive with unresolved material findings.

## End-to-end validation

After audit remediation, run E2E against the real resulting system boundary.

For a user-facing feature:

```yaml
e2e_journey:
  id: <journey ID>
  actor: <real role>
  starting_state: <fixture or precondition>
  entry_point: <page, route, client or command>
  actions:
    - <real action>
  producer_effect:
    - <server, persistence or external effect>
  final_observable_state:
    - <UI/client result>
  persistence_check:
    - <refresh, reload or second-session check>
  negative_path:
    - <validation, authorization or failure behavior>
  result: PASS | FAIL
```

The journey must prove the real frontend/client uses the real backend/system contract, permissions are enforced, valid input succeeds, invalid input fails visibly, expected state persists, and recovery/error states behave correctly.

A backend API test does not replace frontend E2E. A frontend test with mocked data does not replace integration E2E.

For non-UI work, test the complete applicable path:

```text
real input → public/system entry point → processing → persistence/external effect → observable output
```

## E2E unavailable

When required E2E cannot run:

```yaml
e2e:
  result: NOT_RUN
  blocker: <exact blocker>
  attempted: <exact actions>
  required_environment: <exact requirement>
  next_action: <one executable action>
```

Required E2E with `NOT_RUN` prevents `completed`. Use `WAITING`, `BLOCKED`, or an explicitly lower state such as `implementation_complete_unverified` if repository policy permits it.

## Final exact-head CI

After all implementation, remediation, audit and E2E changes:

- resolve the exact final head SHA;
- run every required repository check on that head;
- verify required status names rather than inferring from an earlier head;
- inspect the first relevant failure before rerunning;
- do not merge with queued, stale, missing, skipped-required, or failing checks;
- record the exact run/check evidence.

## Completion evidence

A terminal task should contain:

```yaml
closeout:
  implementation_complete: true
  complete_feature_or_declared_partial: true
  outcome_verified: true
  audit:
    result: PASS
    validator: <identity or session>
    findings_open_material: 0
    evidence:
      - <exact evidence>
  e2e:
    result: PASS | NOT_APPLICABLE_WITH_REASON
    journeys:
      - <journey ID>
    evidence:
      - <exact evidence>
  final_ci:
    head: <sha>
    result: PASS
    checks:
      - <required check>
  pull_requests:
    open_related_prs: 0
    unresolved_review_threads: 0
    terminal_prs:
      - <repo#number state>
  task_archived_or_terminal: true
  ownership_released: true
  stale_branches_reconciled: true
```

## Hard completion gate

Do not mark complete when any is true:

- a required producer or consumer layer is missing;
- frontend/client and backend/system are not integrated;
- outcome evidence relies only on worker narrative;
- a material audit finding remains;
- required E2E failed or was not run;
- final required CI is not green on the exact final head;
- a related PR remains unintentionally open;
- an unresolved review thread or requested change remains;
- the task remains falsely active;
- ownership or leases remain claimed;
- terminal evidence conflicts with live repository state.

## Autonomous continuation

After successful closeout:

1. write terminal evidence;
2. merge or close every remaining related PR;
3. archive or terminally close the task;
4. release ownership, worktree and leases;
5. reconcile programme dependencies and barriers;
6. search for stale PRs or active records left by the completed work;
7. select the next safe `READY` task;
8. continue without routine owner confirmation.

Implementation completion, green CI, merge, audit, E2E, PR cleanup, or task archival are milestones, not automatic programme stop conditions.
