# End-to-End Feature Completeness Contract

## Purpose

A product-facing capability is not complete merely because one producer layer, API, database migration, backend service, protocol, or isolated frontend component exists. Completion means the observable user or system journey works across every applicable layer.

Repository ownership and architecture may split implementation into producer and consumer tasks, but partial tasks must not claim that the complete feature is delivered.

## Required scope classification

Before implementation classify the capability:

```yaml
feature_scope:
  type: full_stack | backend_only | frontend_only | contract_producer | infrastructure | data_pipeline | protocol
  user_facing: true | false
  backend_required: true | false
  frontend_required: true | false
  integration_required: true | false
  e2e_required: true | false
  completion_claim: complete_feature | partial_producer | partial_consumer | internal_only
```

Do not select a partial type merely to reduce work. A partial scope is valid only when architecture or ownership genuinely requires decomposition and every missing consumer has a concrete dependency/task.

## Delivery matrix

For each task mark every applicable layer as `required`, `not_applicable`, or an exact dependent task:

```yaml
delivery_matrix:
  persistence: required
  backend_domain: required
  authorization: required
  validation: required
  api_or_transport_contract: required
  frontend_data_access: required
  frontend_ui: required
  loading_empty_success_error_states: required
  localization: required
  accessibility_and_responsive_behavior: required
  integration: required
  e2e: required
```

`not_applicable` needs a concrete reason. A missing required layer blocks complete-feature status.

## Full vertical slice

A user-facing feature must inspect and implement all applicable layers:

1. schema, persistence, migration and rollback behaviour;
2. backend domain/business rules;
3. server-side authorization and validation;
4. API, controller, action, event, command or transport contract;
5. frontend/client data access and type mapping;
6. reachable page, screen, component, command or interaction;
7. initial, loading, empty, success, validation, authorization, server-error and recovery states;
8. localization and user-visible messages;
9. responsive and accessibility behaviour where applicable;
10. integration using the real producer rather than a permanent mock;
11. focused backend and frontend tests;
12. real end-to-end validation of the observable journey.

An internal feature may replace the UI with another real consumer, but it must still prove the complete input-to-effect-to-observable-output path.

## User-journey acceptance

Acceptance criteria must describe observable behaviour, not only implementation internals.

Weak:

```text
The endpoint returns the new field.
```

Strong:

```text
An authorized user can reach the page, submit a valid value, see an invalid-value error, refresh the page, and observe the persisted value.
```

Every material feature should define:

```yaml
user_journey:
  actor: <role or system>
  starting_state: <precondition>
  entry_point: <route, page, client, command or event>
  actions:
    - <observable action>
  producer_effect:
    - <backend or system effect>
  final_observable_state:
    - <UI, API, persisted or emitted result>
  recovery_or_failure_path:
    - <expected behavior>
```

## Required frontend states

Every interactive consumer must explicitly handle applicable states:

```yaml
frontend_states:
  initial: required
  loading: required
  empty: required_when_applicable
  success: required
  validation_error: required_when_mutating
  authorization_denied: required_when_protected
  network_or_server_error: required
  retry_or_recovery: required_when_safe
```

A happy-path screenshot or isolated component test is not sufficient evidence.

## Contract consistency

Verify producer and consumer agreement on:

- names, types, optionality and nullability;
- enums and state transitions;
- validation limits and error structures;
- authorization and visibility behaviour;
- pagination, sorting and filters;
- date, time, number, currency and locale formats;
- compatibility and rollout order;
- generated or manually maintained client types;
- idempotency and concurrency behaviour where relevant.

Avoid duplicated constants. When duplication is unavoidable, add a deterministic drift check.

## Real integration

The consumer must use the real producer contract in the final integration path.

The following do not prove complete integration:

- backend tests with no reachable consumer;
- frontend mock data with no real backend connection;
- a form that does not persist;
- persistence that is not read back after refresh/reload;
- separate frontend and backend PRs with no integration evidence;
- an endpoint that is inaccessible because permissions or routing are missing;
- a hidden TODO without a registered dependent task.

## Partial delivery

When work is intentionally split, report it explicitly:

```yaml
feature_delivery:
  implementation_status: producer_complete | consumer_complete | integration_complete
  complete_user_facing_feature: false
  missing_consumers:
    - <layer>
  follow_up_tasks:
    - <exact task ID>
  blocked_by:
    - <dependency or none>
```

Use status such as `PRODUCER_COMPLETE`, not `DONE`, for an incomplete product feature. The programme/coordinator remains responsible for the final integrated outcome.

## Evidence

A complete-feature claim requires environment evidence:

```yaml
vertical_slice_evidence:
  backend_checks:
    - <exact command/result>
  frontend_checks:
    - <exact command/result>
  integration_checks:
    - <exact command/result>
  e2e_journeys:
    - <journey ID and result>
  persistence_or_effect_check:
    - <exact evidence>
  remaining_gaps: []
```

Worker narrative is not evidence. Verify changed paths and the resulting environment.

## Coordinator gate

Before archiving a product-facing feature, the coordinator must answer from live state:

1. Are every required producer and consumer implemented?
2. Is the real frontend/client connected to the real backend/system contract?
3. Are required UI and failure states present?
4. Do types, validation, authorization and formats agree?
5. Did the complete user/system journey pass E2E?
6. Are missing layers represented by active dependent tasks rather than hidden TODOs?
7. Does the completion claim match the level actually proven?

If any required answer is no, continue the same vertical slice, start the registered consumer/integration task, or stop with an exact blocker. Do not archive the overall feature as complete.

## Relationship to closeout

This contract decides whether the delivered capability is complete. `TASK_CLOSEOUT_AUDIT_E2E.md` decides whether the completed task has passed independent audit, E2E, final CI, PR hygiene, archival, and ownership release.

Both gates are required for a complete product-facing task.
