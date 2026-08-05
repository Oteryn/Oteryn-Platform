# Oteryn Platform Short Programme Invocation Registry

```yaml
registry_version: 1.2
repository: blakinio/Oteryn-Platform
trusted_base: main
scope_contract: docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
```

## Purpose

The owner may start or resume three durable programmes with a short command. Resolve current work from the canonical prompt, immutable scope contract, mutable programme state, active tasks, Issues, deterministic claim branches, PRs, reviews, CI, ownership and live repository state. Chat history is optional and never authoritative.

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
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
```

Execute the programme instead of returning the long prompt.

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
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
```

Execute the highest-priority safe unclaimed Issue unless the owner explicitly requests a parallel wave. Before product mutation, acquire the deterministic branch `repair/issue-<number>` and activate the Issue/task/PR claim according to protocol version 2.

## Parallel remediation wave

```text
Uruchom 3 agentów naprawczych Platformy autonomicznie.
Uruchom naprawę Platformy równolegle: 3 agentów.
Kontynuuj równoległą naprawę Platformy autonomicznie.
```

Replace `3` with the requested positive worker count.

Resolution rules:

1. Read the immutable Platform scope contract, remediation prompt, programme state, Issue taxonomy and claim protocol.
2. Query live `agent:ready` Issues in `blakinio/Oteryn-Platform` and verify authorization, dependencies, deterministic branches, active claims, tasks and PRs.
3. Reject any Issue whose required mutation belongs outside `blakinio/Oteryn-Platform`; record a Platform-side dependency or blocker instead.
4. Select at most the requested number of Issues that are `parallel_safe`, have distinct coordination keys, non-overlapping exclusive/shared paths and independent migrations/contracts/rollout.
5. Dispatch only the proven safe number; never fill capacity with overlapping, serialized or blocked work.
6. Assign one Issue number to each worker.
7. Each worker posts a provisional visibility marker and attempts to create its exact branch `repair/issue-<number>` in `blakinio/Oteryn-Platform`.
8. GitHub unique-ref creation is the race arbiter. A worker that cannot acquire the ref releases without product mutation and selects another Issue only when authorized.
9. The branch winner creates/updates the active task, opens one draft PR, removes `agent:ready` and posts activation evidence.
10. Record wave, Issues, coordination keys, paths and integration owner in `docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md`.
11. Run a barrier review before shared-path integration or programme-wide closeout.

Coordinator dispatch, Issue comments, labels and assignees are not ownership. The deterministic Git branch is the atomic lock; the Issue and task are visibility and detailed state.

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

Inspect programme state, active tasks, Issues, deterministic claim branches, PRs, reviews and CI in `blakinio/Oteryn-Platform`. Do not mutate unless continuation is also requested.

## Recovery semantics

`Kontynuuj ...` means:

1. read the immutable Platform scope contract, programme state and active task recovery checkpoint first;
2. verify deterministic branch, Issue claim, task, exact head, PR and external operation;
3. reject or block a recovered action if its required mutation is outside `blakinio/Oteryn-Platform`;
4. execute the recorded safe `next_action` immediately when valid;
5. preserve leases, deadlines, counters and recovery generation;
6. take over a stale/orphaned claim only under `SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md` and claim protocol version 2;
7. never ask the owner to paste the full prompt or reconstruct live state.

## Terminal response

```text
STATUS: DONE | WAITING | BLOCKED | ROTATE
RESULT: <observable result>
DURABLE_STATE: <programme/task/Issue/claim branch/head/PR>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```
