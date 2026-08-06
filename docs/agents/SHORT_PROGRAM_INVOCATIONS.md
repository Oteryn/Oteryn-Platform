# Oteryn Platform Short Programme Invocation Registry

```yaml
registry_version: 1.3
repository: blakinio/Oteryn-Platform
trusted_base: main
scope_contract: docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
lifecycle_closeout_contract: docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
```

## Purpose

The owner may start or resume three durable programmes with a short command. Resolve current work from the canonical prompt, immutable scope contract, lifecycle-closeout contract, mutable programme state, active tasks, Issues, deterministic claim branches, PRs, reviews, CI, ownership and live repository state. Chat history is optional and never authoritative.

No command authorizes hidden background execution, production operations, secrets, protected-environment approval, live-data mutation or writes outside `blakinio/Oteryn-Platform`.

The three programme IDs in this registry are permanently dedicated to Oteryn Platform. They cannot be redirected to Otheryn, OTClient, Canary, Freqtrade, Quant Platform, GitHub Projects control or another repository/product area. A request for such work requires a separately named task or programme; it must not be executed by reusing one of these programme IDs.

## Continuous audit

```text
Uruchom audyt Platformy autonomicznie.
Kontynuuj audyt Platformy autonomicznie.
Uruchom OTERYN_PLATFORM_CONTINUOUS_AUDIT autonomicznie.
Kontynuuj OTERYN_PLATFORM_CONTINUOUS_AUDIT autonomicznie.
```

```yaml
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
programme_state: docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
supporting_contracts:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
```

Execute the programme instead of returning the long prompt. A PASS-only independent audit is recorded as a review/comment on the existing target PR and in its audit Issue or durable record; it does not create an audit PR.

## Remediation

```text
Uruchom naprawę Platformy autonomicznie.
Kontynuuj naprawę Platformy autonomicznie.
Uruchom OTERYN_PLATFORM_REMEDIATION autonomicznie.
Kontynuuj OTERYN_PLATFORM_REMEDIATION autonomicznie.
```

```yaml
programme_id: OTERYN_PLATFORM_REMEDIATION
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
programme_state: docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
supporting_contracts:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
```

Execute the highest-priority safe unclaimed Issue unless the owner explicitly requests a parallel wave. Before product mutation, acquire the deterministic branch `repair/issue-<number>` and activate the Issue/task/PR claim according to protocol version 2.

The one-Issue/one-branch/one-PR rule remains mandatory for product, runtime, migration, contract, architecture, dependency, workflow, deployment and security changes. Eligible lifecycle-only/archive-only reconciliations follow `LIFECYCLE_CLOSEOUT_BATCHING.md`: use one bounded coordinator wave PR, one exact-head audit and one CI generation instead of one closeout PR and audit Issue per completed task.

## Parallel remediation wave

```text
Uruchom 3 agentów naprawczych Platformy autonomicznie.
Uruchom naprawę Platformy równolegle: 3 agentów.
Kontynuuj równoległą naprawę Platformy autonomicznie.
```

Replace `3` with the requested positive worker count.

Resolution rules:

1. Read the immutable Platform scope contract, lifecycle-closeout contract, remediation prompt, programme state, Issue taxonomy and claim protocol.
2. Query live `agent:ready` Issues in `blakinio/Oteryn-Platform` and verify authorization, dependencies, deterministic branches, active claims, tasks and PRs.
3. Reject any Issue whose required mutation belongs outside `blakinio/Oteryn-Platform`; record a Platform-side dependency or blocker instead.
4. Route compatible governance-only lifecycle reconciliations to one coordinator batch rather than dispatching one implementation worker per item.
5. Select at most the requested number of product Issues that are `parallel_safe`, have distinct coordination keys, non-overlapping exclusive/shared paths and independent migrations/contracts/rollout.
6. Dispatch only the proven safe number; never fill capacity with overlapping, serialized or blocked work.
7. Assign one product Issue number to each worker.
8. Each product worker posts a provisional visibility marker and attempts to create its exact branch `repair/issue-<number>` in `blakinio/Oteryn-Platform`.
9. GitHub unique-ref creation is the race arbiter. A worker that cannot acquire the ref releases without product mutation and selects another Issue only when authorized.
10. The branch winner creates/updates the active task, opens one draft PR, removes `agent:ready` and posts activation evidence.
11. Record wave, Issues, coordination keys, paths and integration owner in `docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md`.
12. Run a barrier review before shared-path integration or programme-wide closeout.

Coordinator dispatch, Issue comments, labels and assignees are not product ownership. The deterministic Git branch is the atomic product lock; the Issue and task are visibility and detailed state. Lifecycle-closeout batches use the separate coordinator ownership rules in `LIFECYCLE_CLOSEOUT_BATCHING.md`.

## Independent audit and role rotation

A fresh validator audits the exact existing implementation or batch PR head. It submits a PR review/comment and records `PASS_ZERO_MATERIAL_FINDINGS` or exact findings in the linked audit record. Do not create a PR merely to report PASS.

When an implementing session reaches the mandatory independent-audit gate and cannot itself provide an independent validator, leave the checkpoint `ready` with the exact audit `next_action` and return `ROTATE`. Use `WAITING` only for a genuine external dependency, permission, environment, protected operation, observation window, owner decision or exhausted bounded terminal-CI wait.

## Architecture, structure and CI review

```text
Uruchom przegląd architektury Platformy autonomicznie.
Kontynuuj przegląd architektury Platformy autonomicznie.
Uruchom OTERYN_PLATFORM_ARCHITECTURE_REVIEW autonomicznie.
Kontynuuj OTERYN_PLATFORM_ARCHITECTURE_REVIEW autonomicznie.
```

```yaml
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
programme_state: docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
supporting_contracts:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
```

The agent advises and persists proposed/accepted Oteryn Platform architecture documentation and Platform implementation handoffs. It does not change runtime or CI workflow code and never performs architecture work for another repository under this programme ID.

## Status commands

```text
Pokaż stan audytu Platformy.
Pokaż stan napraw Platformy.
Pokaż stan architektury Platformy.
Pokaż stan programów Platformy.
```

Inspect programme state, active tasks, Issues, deterministic claim branches, lifecycle waves, PRs, reviews and CI in `blakinio/Oteryn-Platform`. Do not mutate unless continuation is also requested.

## Recovery semantics

`Kontynuuj ...` means:

1. read the immutable Platform scope contract, lifecycle-closeout contract, programme state and active task recovery checkpoint first;
2. verify deterministic branch or lifecycle-wave ownership, Issue claim, task, exact head, PR and external operation;
3. reject or block a recovered action if its required mutation is outside `blakinio/Oteryn-Platform`;
4. execute the recorded safe `next_action` immediately when valid;
5. preserve leases, deadlines, counters and recovery generation;
6. take over a stale/orphaned claim only under `SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md` and the applicable claim/batching contract;
7. never ask the owner to paste the full prompt or reconstruct live state.

## Terminal response

```text
STATUS: DONE | WAITING | BLOCKED | ROTATE
RESULT: <observable result>
DURABLE_STATE: <programme/task/Issue/claim branch or lifecycle wave/head/PR>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```
