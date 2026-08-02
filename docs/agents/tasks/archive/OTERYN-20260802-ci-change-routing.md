---
task_id: OTERYN-20260802-ci-change-routing
status: completed
programme_issue: 451
issue: 467
implementation_pull_request: 468
implementation_merge_commit: 6af891c47adfba0177372b54419a831b51fa6c09
validated_implementation_head: 2bde126b6e918e9a9d6beb2b6fffa7c06f157790
closeout_pull_request: 469
docs_only_probe_head: f600f32a944a618cae10b6eefba5c743b6452e2e
archived_at: 2026-08-02T14:35:58+02:00
production_state: NOT_CHANGED
---

# OTERYN-20260802 CI change routing

## Goal

Implement deterministic fail-closed pull-request change classification so five previously over-triggered runtime-heavy workflow families skip only proven-unaffected internals while preserving existing terminal job/check identities.

## Terminal result

- A repository-owned classifier covers 13 declared change classes.
- Unknown, mixed, dependency, database/migration, auth/security, payment, deployment, contract, workflow-self and deletion changes fail closed.
- Operational Markdown under `ops/**` and unknown nested Markdown cannot be misclassified as documentation-only.
- Added, copied, modified, renamed and deleted paths participate through diff filter `ACMRD`.
- The five workflows retain original terminal job identities: `test`, `validate` and `concurrency-proof`.
- Job-level routing is used instead of workflow path filters, preserving required-check compatibility.
- Classifier failure runs each original job and fails before heavy validation rather than silently skipping.
- Deterministic fixtures cover positive, negative, mixed and boundary cases.
- Temporary bootstrap workflow and patch helper were removed before implementation validation.

## Implementation validation

Exact implementation head `2bde126b6e918e9a9d6beb2b6fffa7c06f157790` passed:

- Agent Governance — run `30747542044`;
- CI — run `30747542053`;
- Phase 7 Production-Like Validation — run `30747542042`;
- Edge Security Emulation — run `30747542041`;
- Platform DB Outage Validation — run `30747542045`;
- Game Auth Ticket Concurrency — run `30747542043`.

PR #468 merged to `main` as `6af891c47adfba0177372b54419a831b51fa6c09`.

## Real docs-only system proof

PR #469 head `f600f32a944a618cae10b6eefba5c743b6452e2e` changed exactly one file under `docs/agents/**` and passed all six workflow runs.

For every routed workflow, the classifier passed and the original terminal job concluded `skipped` with no steps:

- CI `30748126711`: classifier `91497188125`; `test` `91497217003` skipped;
- Phase 7 `30748126716`: classifier `91497188242`; `validate` `91497205839` skipped;
- Edge `30748126722`: classifier `91497188229`; `validate` `91497208032` skipped;
- DB Outage `30748126727`: classifier `91497188257`; `validate` `91497207026` skipped;
- Game Auth `30748126715`: classifier `91497188186`; `concurrency-proof` `91497202804` skipped;
- Agent Governance `30748126720`: success.

The skipped jobs returned `steps: null`, proving no Composer installation, MariaDB, Redis, MailHog, nginx, Laravel runtime, database outage, edge emulation or game-auth concurrency internals started.

## Independent audit

Result: `PASS`.

The audit checked path precedence, root-versus-nested Markdown, operational markers, contracts, deletions, unknown paths, mixed changes, workflow-self changes, classifier failure behavior, stable job identifiers, temporary instrumentation removal, final diff ownership, reviews and unresolved threads. No critical, high or material medium findings remain.

## E2E classification

Runtime/browser E2E: `NOT_APPLICABLE_WITH_REASON` because this is CI/testing infrastructure. The real system boundary was exercised end to end:

`docs-only Git diff → repository classifier → workflow outputs → original job-level conditions → observable skipped terminal jobs`.

## PR hygiene and ownership

- PR #468: merged.
- PR #469: closeout/proof PR; this archive becomes terminal when it merges after exact-head checks.
- Related reviews: none.
- Unresolved review threads: none at implementation closeout.
- Removal of `docs/agents/tasks/active/OTERYN-20260802-ci-change-routing.md` releases all task ownership.

## Production state

`NOT_CHANGED`. No application behavior, production environment, database content, payment activation, protected secret, deployment target or external repository was modified.
