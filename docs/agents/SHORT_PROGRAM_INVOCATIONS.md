# Oteryn Platform Short Programme Invocation Registry

```yaml
registry_version: 1
repository: blakinio/Oteryn-Platform
trusted_base: main
```

## Purpose

The owner may start or resume the three durable programmes with a short command. The agent must resolve current work from the canonical prompt, mutable programme state, active tasks, Issues, branches, PRs, reviews, CI, ownership and live repository state. Chat history is optional and never authoritative.

No command authorizes hidden background execution, production operations, secrets, protected-environment approval, live-data mutation or writes outside `blakinio/Oteryn-Platform`.

## Continuous audit

Accepted commands:

```text
Uruchom audyt Platformy autonomicznie.
Kontynuuj audyt Platformy autonomicznie.
Uruchom OTERYN_PLATFORM_CONTINUOUS_AUDIT autonomicznie.
Kontynuuj OTERYN_PLATFORM_CONTINUOUS_AUDIT autonomicznie.
```

Resolve through:

```yaml
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
programme_state: docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
supporting_contracts:
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
```

The agent executes the programme rather than returning the long prompt.

## Remediation

Accepted commands:

```text
Uruchom naprawę Platformy autonomicznie.
Kontynuuj naprawę Platformy autonomicznie.
Uruchom OTERYN_PLATFORM_REMEDIATION autonomicznie.
Kontynuuj OTERYN_PLATFORM_REMEDIATION autonomicznie.
```

Resolve through:

```yaml
programme_id: OTERYN_PLATFORM_REMEDIATION
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
programme_state: docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
supporting_contracts:
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
```

The agent executes one highest-priority safe Issue unless the owner explicitly requests a parallel wave.

## Parallel remediation wave

Accepted commands:

```text
Uruchom 3 agentów naprawczych Platformy autonomicznie.
Uruchom naprawę Platformy równolegle: 3 agentów.
Kontynuuj równoległą naprawę Platformy autonomicznie.
```

Replace `3` with the requested positive worker count.

Resolution rules:

1. Read the remediation prompt, programme state, Issue taxonomy and claim protocol.
2. Query live `agent:ready` Issues and verify implementation authorization, dependencies, claims, tasks and PRs.
3. Select at most the requested number of Issues that are all `parallel_safe`, have different coordination keys, non-overlapping exclusive/shared paths and independent rollout/migrations/contracts.
4. If fewer safe Issues exist, dispatch only the proven safe number; never fill capacity with overlapping or blocked work.
5. Assign exactly one Issue number to each worker.
6. Every worker performs the provisional Issue claim and activation protocol before product mutation.
7. Record the wave, Issue numbers, coordination keys, paths and integration owner in `docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md`.
8. Use a barrier review before any shared-path integration or programme-wide closeout.

A coordinator dispatch is not ownership. The first valid Issue claim wins any race.

## Architecture, structure and CI review

Accepted commands:

```text
Uruchom przegląd architektury Platformy autonomicznie.
Kontynuuj przegląd architektury Platformy autonomicznie.
Uruchom OTERYN_PLATFORM_ARCHITECTURE_REVIEW autonomicznie.
Kontynuuj OTERYN_PLATFORM_ARCHITECTURE_REVIEW autonomicznie.
```

Resolve through:

```yaml
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
programme_state: docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
```

The agent advises and persists proposed/accepted architecture documentation and implementation handoffs. It does not change runtime or CI workflow code.

## Status commands

```text
Pokaż stan audytu Platformy.
Pokaż stan napraw Platformy.
Pokaż stan architektury Platformy.
Pokaż stan programów Platformy.
```

For status, inspect the programme state, active tasks, live Issues/claims, branches, PRs, reviews and CI. Return the current state only; do not mutate unless the owner also asks to continue.

## Recovery semantics

`Kontynuuj ...` means:

1. read the programme state and active task recovery checkpoint first;
2. verify live ownership, branch, exact head, PR and external operation;
3. execute the recorded safe `next_action` immediately when still valid;
4. preserve original leases, wait deadlines, counters and recovery generation;
5. recover a stale/orphaned session only under `SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md` and the remediation claim protocol;
6. never ask the owner to paste the full prompt or reconstruct available repository state.

## Terminal response

At a real stop condition, use the canonical compact status from `ANTI_STALL_AND_EXECUTION_BUDGET.md`:

```text
STATUS: DONE | WAITING | BLOCKED | ROTATE
RESULT: <observable result>
DURABLE_STATE: <programme/task/Issue/claim/branch/head/PR>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```
