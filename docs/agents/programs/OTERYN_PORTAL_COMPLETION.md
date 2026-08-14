# Oteryn Portal Completion Programme

```yaml
programme_id: OTERYN_PORTAL_COMPLETION
programme_version: 3
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

`docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md` is the canonical execution-allocation companion for this programme. It maps accepted portal workstreams to model-agnostic execution roles, optional Codex suitability, dependencies and terminal outcomes **after this programme has selected the live work item**.

The companion is not a second scheduler and must not reorder this programme. Role assignment never overrides repository authority or live ownership. `IMPLEMENTATION_OWNER` describes bounded responsibility, not a model choice; execution mode is selected separately. A row marked Codex-suitable is **not permission** to invoke Codex, OpenAI API or any owner-funded AI quota. `AGENTS.md` still requires explicit owner authorization for that exact use/task.

## Live-state rule

This record intentionally does not pin a current branch, PR, Issue claim or `main` SHA. On every invocation the agent must resolve:

- current protected `main` identity;
- active tasks and ownership;
- open and recently relevant Issues/PRs;
- deterministic repair branches;
- reviews, required checks and exact heads;
- merged/superseded work;
- current environment/permission boundaries.

Durable live state overrides every dated example, report, board row and convenience index. `docs/agents/ACTIVE_WORK.md` and `docs/agents/PROJECT_STATE.md` are routing aids only: when they conflict with a newer active task, live PR/Issue state or protected `main`, the live state wins and the stale routing text must not participate in selection.

## Canonical selector states

Every exact candidate is classified from live evidence as exactly one of:

- `TERMINAL` — the required current candidate scope is complete, explicitly deferred/rejected by accepted authority, or no resumable candidate remains;
- `OWNED` — a valid live task/branch/PR/lease already owns the candidate; do not duplicate it;
- `BLOCKED` — an exact dependency, authority, environment or required evidence prevents safe execution;
- `DECISION_REQUIRED` — one explicit owner/product/architecture decision is required before execution can be authorized;
- `READY` — the exact candidate satisfies every eligibility rule below and is unowned.

Classification is per live invocation, not a persistent workstream status. For every skipped earlier entry, persist the exact candidate evidence/reason in the task or selection proof.

### Mixed-entry candidate classification and roll-up

A numbered selection entry may contain more than one exact candidate (for example LiveOps and PublicPortal Today, or Wiki and Game Catalog). The selector must never collapse those candidates before checking whether an unowned `READY` sibling exists.

For each numbered entry:

1. enumerate every exact currently relevant candidate package that can be identified from accepted architecture/programmes, live Issues/tasks/PRs and current dependencies;
2. classify **each candidate** independently as `TERMINAL | OWNED | BLOCKED | DECISION_REQUIRED | READY` with exact evidence;
3. order candidates by an accepted sub-order when one exists; otherwise use dependency order first, then accepted programme/delivery sub-priority, then canonical candidate identity (Issue number ascending, otherwise task/package identifier lexical ascending) only as a final deterministic tie-breaker among otherwise independent peers;
4. roll the entry up with this strict precedence: `READY` if any candidate is `READY`; otherwise `OWNED` if any candidate is `OWNED`; otherwise `DECISION_REQUIRED` if any candidate is `DECISION_REQUIRED`; otherwise `BLOCKED` if any candidate is `BLOCKED`; otherwise `TERMINAL`;
5. when the entry rolls up `READY`, select its first `READY` candidate in the candidate order above before considering any later numbered entry.

`READY` therefore outranks `OWNED` and `BLOCKED` at entry roll-up specifically so an owned or blocked sibling cannot make an independent ready candidate unreachable. The proof must still retain all candidate classifications; roll-up is only the entry-level selector state.

## Selection order

Select the first **safe, unowned, unblocked and implementation-authorized** item that remains unresolved:

1. Resume the current valid portal-completion task when one exists and ownership permits it; otherwise classify the entry `OWNED` or `TERMINAL` with evidence.
2. Reconcile source-of-truth/task/PR/selector drift when it materially prevents reliable selection or closeout.
3. Route any **currently open, implementation-authorized, high-risk** repair Issue through `OTERYN_PLATFORM_REMEDIATION`. Historical examples #948, #944 and #941 are terminal and must never be treated as current queue entries merely because they remain referenced in history.
4. Resolve production/public-edge evidence only with explicit protected-environment authority and access.
5. Complete core account/character lifecycle and ADR 0030 Character Portfolio Platform slices.
6. Deliver LiveOps and public Today; add private Today only after identity/privacy/cache gates. Focused architecture may be selected independently of a blocked runtime producer, but runtime current-state facts require exact authoritative producer evidence.
7. Remove the Announcements/Events reverse edge and deliver federated search.
8. Deliver the accepted first-party Client Distribution Platform boundary, including Issue #1039 or its live successor, while keeping external updater implementation, protected signing and production activation behind their separate gates.
9. Close Wiki/Game Catalog expected-content inventories.
10. Deliver Player Companion P0 vertical slices.
11. Deliver World Hub/community expansions only when authoritative inputs and product need exist.
12. Activate commerce only after its independent security, legal, operational and owner-authorization gates.

Use the work-allocation companion only after the live item has been selected. The companion may choose the execution role and permitted execution mode for that selected item, but it must not select a later board row merely because it is easier, Codex-suitable or already decomposed.

## Eligibility

An item is `READY` only when all of the following are proven from the current trusted state:

- one exact candidate Issue/task package exists or can be created under the new-product-slice rule with evidence-backed problem and acceptance inventory;
- implementation/documentation for that exact package is authorized;
- no valid task/branch/PR/lease already owns the same intent or paths;
- every required predecessor decision/ADR/contract is accepted;
- every authoritative source required by the capability is proven without inventing external producer behavior;
- all intended paths and evidence collection fit Platform repository authority;
- production/protected/external mutations are not implied by completion of the package;
- a complete bounded vertical slice is achievable, or the task is explicitly architecture/documentation-only and does not claim runtime delivery;
- rollback and required focused/integration/E2E/final-CI validation are known.

`ARCHITECTURE_READY` in Work Allocation is **not** canonical `READY`. It means only that an accepted architecture boundary exists. Promotion to canonical `READY` occurs transiently only after every live eligibility condition above is proven. Conversely an `OPEN` or `ARCHITECTURE_READY` workstream may classify `BLOCKED`, `DECISION_REQUIRED` or `OWNED` in the selector.

## LiveOps promotion invariant

Focused LiveOps architecture is owned by `docs/architecture/LIVEOPS_ARCHITECTURE.md`; architecture Issue #1046 is historical terminal evidence, not an active task.

The first runtime package `WorldStatus + configured Maintenance` becomes canonical `READY` only after the exact authoritative runtime-status source needed by the delivered WorldStatus capability is proven from allowed Platform-side evidence. Platform maintenance intent is separate policy authority and cannot manufacture observed runtime health. `ServerSave` remains unavailable/blocked until its own authoritative producer, applicability, time-base, recurrence and freshness semantics are proven. Missing evidence must never become `offline`, `0`, `none`, success or an invented schedule.

`MODULE_CATALOG.md` therefore remains truthful with `LiveOps | PLANNED` until an executable LiveOps capability is merged and validated; architecture completion alone does not promote module implementation availability.

## Routing rules

- Existing audit repair Issue: use the deterministic Issue claim and `OTERYN_PLATFORM_REMEDIATION` contract.
- New product slice: create or reuse one canonical Issue, one task record, one branch and one PR under current governance.
- Existing authoritative PR: reuse/fix/rebase it when safe; do not create a duplicate.
- Superseded/duplicate/obsolete PR: close only with concrete proof and preserve unique work/evidence.
- Server/game dependency: mark the exact candidate `BLOCKED` or `DECISION_REQUIRED` as appropriate; do not inspect an external repository without separate owner permission.
- Production dependency: perform only repository-safe work, persist the exact operator gate and select another independent safe item when possible.
- Specialized programme dependency: route detailed decomposition through that programme without changing this selection order; for Game Catalog this includes `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md`.

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
- the programme queue is re-evaluated from the new protected `main`.

## Real stop conditions

Return a terminal status only for:

- `DONE`: the selected item and required closeout are terminal;
- `WAITING`: a genuine accepted external actor, permission/environment, protected operation, observation window or bounded terminal-CI wait;
- `BLOCKED`: no safe progress exists because an exact decision, authority, evidence or dependency is missing;
- `ROTATE`: durable next action requires another eligible session/task boundary under repository execution-budget rules.

Do not use `WAITING` for another internal worker, reviewer, auditor or hypothetical future task.
