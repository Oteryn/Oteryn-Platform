---
programme_id: OTERYN_PLATFORM_REMEDIATION
programme_version: 6
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
required_reads:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
repository: Oteryn/Oteryn-Platform
ownership_model: one_issue_one_owner_end_to_end
external_repair_audit: disabled
---

# Oteryn Platform Remediation — Programme State

## Mission

Consume confirmed implementation-authorized findings through one accountable implementation owner per Issue, complete the required vertical slice, perform documented exact-head self-review, run risk-proportional validation and applicable real E2E, satisfy exact-head CI, then merge and complete terminal closeout.

A different-agent PASS is not required for repair merge. The separate continuous-audit programme may inspect the platform and create new Issues, but it does not receive every completed repair for certification.

## Worker lifecycle

Each worker:

1. queries live repository state and selects one eligible unclaimed repair Issue;
2. acquires the deterministic claim branch and activates the task record;
3. becomes the sole implementation owner for that Issue;
4. analyses root cause, acceptance, dependencies, rollback and compatibility;
5. implements the smallest complete repair;
6. runs focused tests during construction;
7. builds a coherent reviewable candidate before pushing broad changes;
8. performs exact-head full-diff self-review;
9. completes applicable E2E and repository-required exact-head CI;
10. repairs findings from self-review, CI or ordinary PR review;
11. merges through normal branch protection;
12. verifies resulting state, closes the Issue, archives the task and releases ownership;
13. selects another safe Issue only when the execution budget permits.

Do not hand the repair to another agent for approval. Do not create an audit Issue, audit PR, frozen audit generation or idle auditor slot.

## Validation intensity

Use `REMEDIATION_AUDIT_RISK_GATE.md` as the compatibility path for the remediation self-review and risk gate:

- `STANDARD` for bounded reversible low/medium repairs with deterministic evidence;
- `HEIGHTENED` for critical/high, security, payment, integrity, concurrency, migration, public contract, dependency, CI/deployment, production, architecture or cross-repository boundaries;
- `BLOCKED` for unresolved authority, rollback, compatibility, environment, ownership, `UNKNOWN` or `CONFLICT`.

Heightened validation means stronger tests and evidence, not mandatory certification by another agent.

## Parallelism

A request for `N` repair agents launches up to `N` end-to-end Issue owners. Each claims a distinct Issue. No auditor or integrator slot is permanently reserved.

A worker that loses a claim immediately selects another eligible Issue. Workers do not wait to fill a train or wait for an internal role.

## PR and Actions economy

- one ordinary Issue uses one authoritative delivery PR;
- batch coherent edits before pushing;
- do not create one commit per file, checkpoint, comment or evidence line;
- use focused validation during construction and one full applicable final pass;
- cancel superseded same-PR workflow runs;
- do not start unrelated heavy runtime workflows for task/checkpoint/governance-only changes;
- avoid unnecessary merge-base refreshes and no-op heads;
- use squash merge unless repository policy requires another method.

## Repair trains

Ordinary product/runtime repairs do not use repair trains. Lifecycle-only reconciliation may be batched when homogeneous, reversible and provenance-preserving. A coherent repair never waits merely to fill a batch.

## Durable programme state

```yaml
programme_state_version: 6
updated_at: 2026-08-06T19:20:00Z
status: ready
live_state_snapshot:
  mode: live_query_required
  exhaustive: false
selection:
  unit: one_issue_per_worker
  ownership: end_to_end
  reserved_audit_slots: 0
closeout:
  external_repair_audit_required: false
  exact_head_self_review_required: true
  exact_head_ci_required: true
  applicable_e2e_required: true
  issue_archive_release_required: true
actions_economy:
  coherent_pushes: required_when_practical
  superseded_run_cancellation: required
  heavy_runtime_path_routing: required
```

## Stop conditions

Stop or rotate only for a real blocker, exhausted execution budget, unresolved authority/safety issue, ownership conflict, external dependency, required environment unavailable, or material `UNKNOWN`/`CONFLICT` that prevents safe completion.

Pending ordinary CI, routine findings repair, merge, archival and next-task selection are not reasons to ask the owner for routine confirmation.
