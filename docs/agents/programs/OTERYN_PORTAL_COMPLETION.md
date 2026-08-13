# Oteryn Portal Completion Programme

```yaml
programme_id: OTERYN_PORTAL_COMPLETION
programme_version: 1
repository: blakinio/Oteryn-Platform
trusted_base: main
owner_alias: PORTAL-CLOSEOUT
status: READY_FOR_LIVE_SELECTION
canonical_prompt: docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
delivery_plan: docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
work_allocation: docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
architecture_owner: docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
repair_programme: OTERYN_PLATFORM_REMEDIATION
communication: terminal_only
live_state_required: true
production_authority: false
external_repository_authority: false
codex_standing_permission: false
```

## Mission

Close the Oteryn web portal from its current strong foundation to a secure, operable and player-useful product by executing one bounded, highest-priority ready slice at a time through full validation and terminal lifecycle closeout.

This programme is an orchestrator and delivery queue. It does not duplicate:

- `OTERYN_PLATFORM_CONTINUOUS_AUDIT`, which discovers and deduplicates findings;
- `OTERYN_PLATFORM_REMEDIATION`, which owns an implementation-authorized repair Issue end to end;
- accepted architecture and operation-specific contracts;
- production or cross-repository authorization.

## Work allocation

`docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md` is the canonical execution-allocation companion for this programme. It maps the accepted P0-P3 portal backlog to execution roles, Codex suitability, dependencies and terminal outcomes.

Role assignment never overrides repository authority or live ownership. In particular, `CODEX_ELIGIBLE_WORKER` means only that a bounded task is technically suitable for Codex-style implementation. It is **not permission** to invoke Codex, OpenAI API or any owner-funded AI quota. `AGENTS.md` still requires explicit owner authorization for that exact use/task.

## Live-state rule

This record intentionally does not pin a current branch, PR, Issue claim or `main` SHA. On every invocation the agent must resolve:

- current protected `main` identity;
- active tasks and ownership;
- open and recently relevant Issues/PRs;
- deterministic repair branches;
- reviews, required checks and exact heads;
- merged/superseded work;
- current environment/permission boundaries.

Durable live state overrides the dated queue below.

## Selection order

Select the first **safe, unowned, unblocked and implementation-authorized** item that remains unresolved:

1. Resume the current valid portal-completion task when one exists.
2. Reconcile source-of-truth/task/PR drift when it materially prevents reliable selection or closeout.
3. Route ready high-risk repair Issues through `OTERYN_PLATFORM_REMEDIATION`, currently expected to include the live successors of:
   - #948 — immutable client-release artifact identity;
   - #944 — bounded entitlement stale-authority lease;
   - #941 — owner-private Today cache isolation.
4. Resolve production/public-edge evidence only with explicit protected-environment authority and access.
5. Complete core account/character lifecycle and ADR 0030 Character Portfolio Platform slices.
6. Deliver LiveOps and public Today; add private Today only after identity/privacy/cache gates.
7. Remove the Announcements/Events reverse edge and deliver federated search.
8. Close Wiki/Game Catalog expected-content inventories.
9. Deliver Player Companion P0 vertical slices.
10. Deliver World Hub/community expansions only when authoritative inputs and product need exist.
11. Activate commerce only after its independent security, legal, operational and owner-authorization gates.

Use the work-allocation companion to choose the execution role after the live item has been selected. Do not select a Codex worker merely because a row is marked Codex-suitable; explicit owner authorization is still required for each owner-funded AI invocation.

## Eligibility

An item is `READY` only when:

- the exact problem and acceptance inventory are evidence-backed;
- implementation is explicitly authorized in the current trusted state;
- no valid task/branch/PR/lease already owns it;
- required predecessor decisions/contracts are accepted;
- all intended paths fit Platform repository authority;
- production/protected/external mutations are not implied;
- a complete bounded vertical slice is achievable or the task is explicitly architecture/documentation-only;
- rollback and required validation are known.

## Routing rules

- Existing audit repair Issue: use the deterministic Issue claim and `OTERYN_PLATFORM_REMEDIATION` contract.
- New product slice: create or reuse one canonical Issue, one task record, one branch and one PR under current governance.
- Existing authoritative PR: reuse/fix/rebase it when safe; do not create a duplicate.
- Superseded/duplicate/obsolete PR: close only with concrete proof and preserve unique work/evidence.
- Server/game dependency: mark `CROSS-REPOSITORY ARCHITECTURE DECISION REQUIRED`; do not inspect an external repository without separate owner permission.
- Production dependency: perform only repository-safe work, persist the exact operator gate and select another independent safe item when possible.

## Completion state

A selected item becomes terminal only after:

- implementation/documentation is complete for its declared scope;
- exact-head full-diff self-review has no open material finding;
- risk-proportional tests pass;
- real E2E is `PASS` or concretely `NOT_APPLICABLE` for non-executable scope;
- required checks pass on the exact final head;
- review threads and related PRs are intentional and terminal;
- the PR is merged when authorized and eligible;
- the Issue/task is closed or archived;
- ownership and leases are released;
- the programme queue is re-evaluated from live state.

## Real stop conditions

Return a terminal status only for:

- `DONE`: the selected item and required closeout are terminal;
- `WAITING`: a genuine accepted external actor, permission/environment, protected operation, observation window or bounded terminal-CI wait;
- `BLOCKED`: no safe progress exists because an exact decision, authority, evidence or dependency is missing;
- `ROTATE`: durable next action requires another eligible session/task boundary under repository execution-budget rules.

Do not use `WAITING` for another internal worker, reviewer, auditor or hypothetical future task.
