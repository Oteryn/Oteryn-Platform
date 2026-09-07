# Oteryn Platform Architecture, Structure and CI Review Programme

```yaml
prompt_contract:
  version: 2.0.0
  programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
  objective: Resolve one bounded Platform architecture, repository-structure or CI decision with evidence, alternatives and an explicit implementation handoff.
  baseline_version: 1.0.0
  eval_suite: docs/agents/evals/prompt-contract-v1.json
  rollback_version: 1.0.0
  required_invariants:
    - review_does_not_implement_runtime
    - proposed_decision_is_not_authority
    - material_decision_compares_alternatives
  changed_surfaces:
    - architecture review scope
    - decision classification
    - implementation handoff
```

This reusable prompt is a Platform task delta under the bound META organization policy and repository bootstrap. Resolve live architecture, ADR, contract, programme, Issue, task, PR and CI state at invocation.

## Outcome and authority

Advise within `Oteryn/Oteryn-Platform`. Runtime implementation is outside this programme. Writes are limited to bounded review records, deduplicated decision Issues, proposed ADRs/contracts, and narrow canonical documentation updates when an accepted decision already exists.

Do not silently choose product policy, provider, protocol, data owner, trust boundary or cross-repository rollout while material alternatives remain. A proposed ADR grants no implementation authority. External repository mutation, production/deployment operations, credentials, live data and irreversible effects require separate exact authority.

Use `docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md` as durable state. Route accepted implementation to `OTERYN_PLATFORM_REMEDIATION`.

## Review method

Choose one highest-risk unresolved question after checking current ownership and duplicates. Establish the observed state from primary evidence and preserve `PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT`.

Review applicable concerns:

- bounded contexts, module ownership, dependency direction and duplicate policy;
- API/event/queue/command contracts and producer/consumer ownership;
- persistence authority, consistency, transactions, idempotency and reconciliation;
- security trust zones, least privilege, sessions, secrets and auditability;
- scaling, backpressure, availability, recovery, rollback and operability;
- versioning, compatibility, rollout order and provider neutrality;
- repository paths, generated artifacts, stale indexes and conflicting canonical sources;
- deterministic builds, workflow permissions, test layering, change detection, exact-head checks, release and rollback proof.

Classify the item as `defect`, `missing_decision`, `contradiction`, `improvement`, `documentation_drift`, `not_applicable` or `false_positive`. For a material decision, compare at least two viable alternatives, including the status quo when viable, across security, correctness, complexity, scalability, migration, operability, cost, coupling, reversibility and delivery risk.

A recommendation records decision ID, current state, invariant/problem, impact, constraints, options and trade-offs, confidence, rejected alternatives, security/data/contract implications, rollout/rollback, implementation owners and acceptance. If evidence cannot select safely, state the exact decision owner and blocking question.

## Durable decision delta

Search before creating an Issue or ADR. Use an ADR only for a decision expected to outlive one task, discover live numbering, and mark a new ADR `Proposed`. Keep authoritative conflicts visible until evidence resolves them. Avoid creating a second registry for an existing canonical concern.

For a CI defect, record the exact workflow/job/check/trigger/permissions/current behavior and distinguish code failure, workflow defect, ruleset mismatch and external infrastructure failure. Define the smallest proving acceptance and hand it off; do not edit the workflow in the architecture-review task.

## Acceptance delta

- Current state and claims are grounded in primary evidence.
- Existing decisions, Issues, tasks and PRs are deduplicated.
- Security, data, contracts, scalability, reliability, operability and delivery consequences are classified where applicable.
- Material decisions include alternatives and trade-offs.
- Proposed and accepted states remain distinct; unknowns and conflicts remain explicit.
- Implementation ownership, rollout/rollback and validation expectations are actionable.
- Exact-head documentation/reference checks and task lifecycle are coherent.

Runtime/browser E2E for a review-only diff is `NOT_APPLICABLE` because it changes no executable behavior. Persist the chosen question, result, decision state, handoff and one next action in the canonical terminal response.
