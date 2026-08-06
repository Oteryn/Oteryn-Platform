# Oteryn Platform Short Programme Invocation Registry

```yaml
registry_version: 1.4
repository: blakinio/Oteryn-Platform
trusted_base: main
scope_contract: docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
repair_delivery_contract: docs/agents/REPAIR_PR_ECONOMY.md
lifecycle_closeout_contract: docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
```

## Purpose

The owner's established short commands remain valid. Resolve work from canonical prompts, immutable scope, repair-delivery and lifecycle contracts, mutable programme state, active tasks, Issues, deterministic claim branches, PRs, reviews, CI, ownership and live repository state. Chat history is optional and never authoritative.

No command authorizes hidden background execution, production operations, secrets, protected-environment approval, live-data mutation or writes outside `blakinio/Oteryn-Platform`.

The three programme IDs remain permanently dedicated to Oteryn Platform and cannot be redirected to Otheryn, OTClient, Canary, Freqtrade, Quant Platform or another repository.

## Continuous platform audit

```text
Uruchom audyt Platformy autonomicznie.
Uruchom audyt całej Platformy autonomicznie.
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
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
```

Execute the programme instead of returning its long prompt. PASS-only audits are reviews/comments on existing target PRs, never standalone PASS PRs.

## Platform remediation

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
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
```

Select the highest-priority safe unclaimed Issue, acquire `repair/issue-<number>`, activate branch/Issue/task ownership and apply the mandatory delivery selection order. A draft PR is not required merely because the claim became active.

## Independent audit of ready repairs

This is a role invocation within `OTERYN_PLATFORM_REMEDIATION`, not a fourth product programme.

```text
Uruchom niezależny audyt gotowych napraw Platformy autonomicznie.
Kontynuuj niezależny audyt gotowych napraw Platformy autonomicznie.
Uruchom audyt napraw Platformy autonomicznie.
```

Resolution rules:

1. Query active remediation tasks and delivery PRs for durable `audit_handoff` records with checkpoint `status: ready`.
2. Select the oldest highest-priority valid handoff whose exact PR/base/head still matches live state.
3. Verify the auditor is distinct from the implementation owner, all contributing workers and train integration owner, and did not write/remediate target commits.
4. Operate in `AUDIT ONLY`; do not mutate the target branch.
5. Record a whole-diff verdict and one verdict per included Issue on the exact target PR.
6. PASS requires zero material findings and every per-Issue verdict PASS.
7. A target change invalidates the generation; findings return to the implementation owner on the same delivery PR unless a separate root cause is proven.
8. Do not create an audit PR merely to record PASS.

## Parallel remediation workers

Literal implementation-worker command:

```text
Uruchom 3 agentów naprawczych Platformy autonomicznie.
Uruchom naprawę Platformy równolegle: 3 agentów.
```

Here `3` means up to three implementation workers. Each worker independently acquires one deterministic Issue branch. The coordinator dispatches only the proven safe number and does not fill capacity with blocked or overlapping work.

Recommended total-slot command:

```text
Uruchom 3 sloty naprawy Platformy autonomicznie.
Uruchom równoległą naprawę Platformy w 5 slotach autonomicznie.
```

Here the number is total concurrent roles. Default allocation:

```yaml
slot_allocation:
  2:
    repair_workers: 1
    audit_workers: 1
  3:
    repair_workers: 2
    audit_workers: 1
  4:
    repair_workers: 2
    audit_workers: 1
    integration_coordinator: 1
  5:
    repair_workers: 3
    audit_workers: 1
    integration_coordinator: 1
```

For larger counts, preserve at least one audit role when ready audit handoffs exist and add an integration coordinator when repair trains/shared paths require one. Never dispatch more implementation workers than proven independent ready Issues.

### Parallel resolution

1. Read Platform scope, repair economy, lifecycle, remediation prompt/state, Issue taxonomy and claim protocol.
2. Query live ready/claimed Issues, deterministic branches, tasks and related PRs.
3. Reject unauthorized, blocked, overlapping or external-repository mutations.
4. Route compatible terminal governance-only items to lifecycle batching.
5. Select independent product Issues with distinct coordination keys and non-overlapping paths.
6. Each worker posts provisional visibility and attempts its exact branch once.
7. A losing worker releases immediately and selects another eligible Issue when authorized.
8. Branch winners activate task ownership; PR creation follows `REPAIR_PR_ECONOMY.md`, not an activity requirement.
9. Compatible coherent candidates may enter a repair train only through exact immutable source-head handoff to one integration owner.
10. A worker never waits for another worker, train capacity or auditor. It persists a durable handoff and returns `ROTATE` when role transition is required.
11. The audit role drains valid ready handoffs independently.

Coordinator dispatch, comments, labels and assignees do not acquire Issue ownership. The deterministic branch remains the atomic lock.

## Architecture review

```text
Uruchom przegląd architektury Platformy autonomicznie.
Uruchom audyt architektury Platformy autonomicznie.
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

The agent analyses and persists proposed/accepted Platform architecture decisions and implementation handoffs. It does not become the final independent auditor of a repair it implemented or integrated.

## Status commands

```text
Pokaż stan audytu Platformy.
Pokaż stan napraw Platformy.
Pokaż kolejkę audytów napraw Platformy.
Pokaż stan architektury Platformy.
Pokaż stan programów Platformy.
```

Status commands inspect programme state, active tasks, Issues, deterministic branches, repair trains, audit handoffs, lifecycle waves, PRs, reviews and CI without mutation unless continuation is requested.

## Recovery

`Kontynuuj ...` means:

1. read immutable scope, repair/lifecycle contracts, programme state and task recovery checkpoint;
2. verify branch/Issue/task/PR/audit handoff/external-operation live state;
3. reject unauthorized external mutations;
4. execute the recorded safe `next_action`;
5. preserve leases, deadlines, counters, accepted source heads, audit generations and recovery generation;
6. take over stale/orphaned work only under recovery and claim contracts;
7. never ask the owner to paste the long prompt.

## Status semantics

- `ROTATE`: current role completed its bounded phase and a distinct role/session must execute the durable next action.
- `WAITING`: genuine external dependency, accepted external actor, permission/environment, protected operation, observation window, owner decision or exhausted bounded terminal-CI procedure.
- An implementer needing an independent auditor returns `ROTATE`, not `WAITING`.
- A coherent repair never waits merely to fill a train.

## Terminal response

```text
STATUS: DONE | WAITING | BLOCKED | ROTATE
RESULT: <observable result>
DURABLE_STATE: <programme/task/Issue/branch/head/PR/train/audit generation>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```
