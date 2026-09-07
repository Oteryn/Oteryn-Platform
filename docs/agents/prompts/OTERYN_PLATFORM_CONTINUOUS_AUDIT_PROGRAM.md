# Oteryn Platform Continuous Audit Programme

```yaml
prompt_contract:
  version: 2.0.0
  programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
  objective: Falsify technical-correctness and end-to-end-completeness claims for one bounded Platform domain and create durable, deduplicated remediation evidence.
  baseline_version: 1.0.0
  eval_suite: docs/agents/evals/prompt-contract-v1.json
  rollback_version: 1.0.0
  required_invariants:
    - audit_does_not_implement_findings
    - unknown_claims_remain_explicit
    - deduplicate_finding_handoff
  changed_surfaces:
    - audit scope
    - finding lifecycle
    - remediation handoff
```

This reusable prompt is a Platform task delta under the bound META organization policy and the repository bootstrap. Resolve live state at invocation; do not treat examples or prior audit conclusions as current facts.

## Outcome and authority

Independently audit one highest-risk eligible domain in `Oteryn/Oteryn-Platform`. Writes are limited to audit records, programme state, deduplicated finding Issues, and documentation-only missing-module proposals. Do not implement product, runtime, migration, dependency, workflow, deployment or infrastructure fixes in the audit task.

External repositories and systems are read-only unless separately authorized. Production deployment, credentials, live data and irreversible external actions require separate exact authority. An auditor must not claim independent validation of its own implementation.

Use `docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md` as durable programme state. Route confirmed repair through `OTERYN_PLATFORM_REMEDIATION` and missing architecture decisions through `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`.

## Audit method

Select one coherent package after checking current main, active ownership, PRs, Issues and prior findings. Inspect primary code, configuration, contracts and exact environment evidence. Preserve `PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT`; do not convert a mocked or prior-head result into delivery proof.

Classify the subject across its applicable layers:

- routes, commands, events, queues, schedulers, APIs, webhooks and integrations;
- persistence, migrations, rollback, transactions, locking, idempotency and data ownership;
- domain correctness, authorization, tenancy, validation and failure handling;
- real frontend reachability and loading, empty, success, denied, error, stale and recovery states;
- localization, accessibility, responsive behavior and browser compatibility;
- authentication, sessions, uploads, rate limits, secrets, audit logging and untrusted content;
- cross-repository contract compatibility, rollout order, downgrade and fail-closed behavior;
- unit, integration, contract, migration, security and real E2E evidence;
- configuration, CI, release, observability, backup and recovery;
- implementation/documentation drift, dead code and unsupported completion claims.

Prioritize security, authorization, data loss, financial/session risk, recent user-facing changes, partial integrations, false-green CI and recovery gaps. Use focused negative, boundary, concurrency, authorization, failure and recovery checks proportionate to the claim.

## Finding and handoff delta

Each finding records a stable ID, severity, confidence, evidence state, exact location or reproducible behavior, expected/actual result, impact, affected delivery layers, remediation acceptance, dependencies, rollback implications, duplicate search and disposition.

Search open and closed Issues, tasks and PRs before creating an Issue. One actionable root cause or tightly coupled defect cluster gets one Issue. Do not close it merely because a PR exists; verify the merged outcome.

For an entirely absent required subsystem, prove the accepted need, search for an existing decision or implementation, create one finding Issue, and if useful create a separate proposed documentation design covering responsibility, data/security boundaries, producer/consumer contracts, rollout/rollback and validation. Mark it proposed until accepted. The audit task does not implement it.

## Acceptance delta

- Every in-scope surface is inventoried once and each applicable delivery layer has an evidence state.
- Material claims use primary evidence; unknowns remain explicit.
- Negative, failure and recovery behavior is checked in proportion to risk.
- Findings are deduplicated and each confirmed material gap has one durable path.
- The diff contains no product/runtime repair.
- Audit records, Issue links, exact-head validation and task lifecycle are coherent.

Runtime E2E for an audit-document-only diff is `NOT_APPLICABLE` because it changes no executable behavior. E2E can still be mandatory evidence for the product claim being audited. Persist the selected domain, coverage, open findings and one next action in the canonical terminal response.
