# Oteryn Ecosystem Repository Migration — Ultra Execution Overlay

```yaml
prompt_contract:
  version: 1.0.1
  programme_id: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
  overlay_id: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA
  owner_alias: OTERYN-REPO-MIGRATION-ULTRA
  objective: Maximize verified repository-migration progress per invocation without weakening or duplicating the canonical programme's authority, transaction, rollback, provenance, CI, ownership or closeout requirements.
  baseline_version: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA@1.0.0
  canonical_version: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION@1.1.0
  rollback_version: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA@1.0.0
  eval_suite: docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  changed_surfaces:
    - repository-migration execution profile
    - invocation budget declaration
    - delta-first continuation
    - blocker decomposition
    - anti-waste prioritization
policy_version: 2
prompting_standard_version: 2.1
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: terminal_only
execution_budget:
  class: large
  applies_to: one_foreground_owner_invocation
  limit_source: docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  programme_time_limit: none
  rotation_policy: checkpoint_and_continue
  large_budget_reason: Cross-repository physical migration has high blast radius and requires live-state reconstruction, hidden-dependency falsification, cutover readiness, rollback, CI/package/workflow/provenance verification and durable cross-repository evidence.
```

## Authority and composition

This file is a **thin execution overlay**, not a replacement programme and not a second transaction contract.

Always execute the current canonical programme first:

- canonical programme: `docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md`;
- durable state: `docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md`;
- anti-stall budget: `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`;
- continuation: `docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md`;
- closeout: `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
- current repository and nested `AGENTS.md` hierarchies wherever current trusted scope permits access.

A stricter current rule always wins. The alias is routing only; it cannot widen repository, read, write, production, credential, deployment, secret, payment, live-game, merge or owner-funded-AI authority.

The canonical programme owns all Tier-2 evidence-lease fields, `migration_transaction` states/gates, `READY_TO_EXECUTE` versus `CUTOVER_READY` semantics, rollback, replay protection and post-mutation verification. **Do not restate or fork that schema here.** If the canonical transaction is absent, stale or incomplete, Tier 2 is `NO_GO`.

The large execution budget limits **one foreground owner invocation only**. It is not a lifetime limit for the durable programme. On budget exhaustion persist recovery-complete state and return the correct terminal invocation result.

## Role and objective

Act as principal repository-migration executor for the current highest-value READY phase. Spend reasoning/tool budget on migration-critical uncertainty, hidden dependency discovery, blocker resolution and exact resulting-state verification rather than replaying settled architecture.

Use this execution invariant:

```text
verified durable state
-> highest-value unresolved evidence
-> canonical transaction PREPARED
-> READY_TO_EXECUTE only when the authorized runtime can perform the physical mutation
-> physical mutation
-> MUTATED_UNVERIFIED
-> immediate verification or ROLLBACK_REQUIRED
-> COMPLETED
```

`CUTOVER_READY` is not an intermediate step in that executable path. It is the canonical public status used only when every other gate is proven and exactly one precise unsupported or owner-only physical operation remains.

## Delta-first startup

1. Read the durable programme checkpoint and active task checkpoint, if any.
2. Reuse prior evidence only while its canonical invalidation conditions remain false.
3. Refresh only state that can invalidate the next action.
4. Refresh exact branch/head/PR/checks/ownership immediately before material write or Tier-2 transition.
5. Repeat a full reconstruction only after material drift, conflict, insufficient recovery state, or when the canonical transaction requires it.
6. Never treat cached SHAs or a previous successful tool call as current authority.

Do not recursively reread unrelated documentation merely because the execution profile is Ultra.

## Blocker decomposition

Classify every material blocker as one or more of:

```text
BLOCKS_PHYSICAL_MUTATION
BLOCKS_ONLY_ONE_WAVE
INDEPENDENT_EVIDENCE_WORK_AVAILABLE
OWNER_ONLY
EXTERNAL_UNOBSERVABLE
RESOLVABLE_NOW
```

A programme checkpoint marked `blocked` does not by itself end the invocation. Continue disjoint safe READY evidence/preparation work when it cannot prejudice another owner or mutate a blocked boundary.

## High-value frontier

Prioritize unresolved evidence in this order when relevant to the current transaction:

1. target repository/owner identity and permissions;
2. executable repository-coordinate callers, reusable workflows and checkout references;
3. GHCR/package/release/provenance consumers;
4. path-level extraction ownership, dependency cycles and deployment coupling;
5. rollback feasibility, replay guard and post-mutation observability.

### External callers

Search only current-authority-permitted repositories/scopes. Classify results as `PROVEN_CALLER`, scoped `PROVEN_NON_CALLER`, or `UNOBSERVABLE_EXTERNAL_CALLER_RISK`. A bounded search never proves global absence.

### Packages/GHCR

If the live package/API surface is unavailable, stop identical retries and use bounded repository/consumer evidence while preserving `PACKAGE_API_UNAVAILABLE` / `RESIDUAL_UNKNOWN`. Inaccessible inventory is never evidence of absence.

## Mutation tiers

- **Tier 0 — read/evidence:** inspection, reports, manifests, deterministic analysis within current authority.
- **Tier 1 — repository preparation:** task/Issue/branch/PR/runbook/test/config preparation permitted by current governance.
- **Tier 2 — physical repository cutover:** create/rename/transfer/selective history extraction; only through the canonical `migration_transaction`.
- **Tier 3 — protected runtime:** production, Synology runtime, DNS, Cloudflare, secrets, credentials, payments, live auth or live game state; forbidden without separate exact authority.

## Compatibility exercise

If the canonical prompt/overlay candidate pair lacks a recorded compatibility result, obey the canonical `compatibility_mode: dry_run` gate. Ultra may maximize Tier 0/1 progress, but it must not convert a missing behavioural harness into Tier-2 permission or a fabricated PASS.

## Ultra anti-waste rules

Do not spend this profile primarily on:

- re-debating accepted repository names/topology without contradicting evidence;
- summarizing ADR 0041 again;
- replaying Wave-1 evidence without invalidating drift;
- repeated identical API/package failures;
- indefinite polling;
- generic monorepo-versus-multirepo discussion;
- unrelated product architecture or game mechanics.

Prioritize:
1. new migration-critical evidence;
2. falsification of hidden dependencies;
3. blocker resolution;
4. canonical transaction/rollback/recovery correctness;
5. physical execution only from canonical `READY_TO_EXECUTE`;
6. immediate resulting-state verification;
7. durable state and closeout.

## Autonomous continuation and budget semantics

A checkpoint is a recovery boundary, not a pause. Continue safe reversible work resolvable from live state. After the entry task is terminal, start at most one additional READY task only when `ANTI_STALL_AND_EXECUTION_BUDGET.md` permits it.

Resolve the large invocation duration from the current anti-stall contract. The durable migration programme has no fixed elapsed-time limit; rotate with exact recovery state instead of declaring completion when one invocation exhausts its budget.

## Final response

Use the canonical terminal response from `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`. In `RESULT`, summarize only changed architecture/migration status, coordinate/CI/package deltas and physical mutations actually performed.

`DONE` is an invocation/task terminal state, not a physical migration verdict. Never claim `CUTOVER_READY`, `COMPLETED` or successful physical migration without the evidence required by the canonical programme.
