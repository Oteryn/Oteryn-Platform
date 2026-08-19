# Oteryn Portal Completion Programme

```yaml
programme_id: OTERYN_PORTAL_COMPLETION
programme_version: 4
repository: Oteryn/Oteryn-Platform
trusted_base: main
owner_alias: PORTAL-CLOSEOUT
status: READY_FOR_LIVE_SELECTION
canonical_prompt: docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
delivery_plan: docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
completion_scope: docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json
work_allocation: docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
architecture_owner: docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
architecture_review_programme: docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
repair_programme: OTERYN_PLATFORM_REMEDIATION
communication: terminal_only
live_state_required: true
production_authority: false
external_repository_authority: false
codex_standing_permission: false
```

## Mission

Close the Oteryn web portal from its current strong foundation to a secure, operable and player-useful product by selecting and completing the highest-priority safe live candidate through full validation and terminal lifecycle closeout.

**This document is the sole live portal selector.** One selector pass chooses at most one new candidate for the current worker/invocation entry. Independently owned, non-overlapping portal tasks may exist and execute globally in parallel under repository multi-agent rules; their existence is represented as `OWNED` and never authorizes this selector to create duplicate ownership.

This programme is an orchestrator and delivery queue. It does not duplicate:

- `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`, which owns unresolved durable architecture decisions and accepted architecture handoffs;
- `OTERYN_PLATFORM_CONTINUOUS_AUDIT`, which discovers and deduplicates findings;
- `OTERYN_PLATFORM_REMEDIATION`, which owns an implementation-authorized repair Issue end to end;
- accepted architecture and operation-specific contracts;
- repository branch/history governance;
- production or cross-repository authorization.

## Control-plane hierarchy

Portal delivery uses one explicit hierarchy. Lower layers may add evidence and detail but may not reorder or expand a higher layer's authority:

1. `docs/architecture/ROADMAP.md` — global Platform phase/risk ordering and phase exit gates; not a live portal queue.
2. `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md` — canonical portal completion/release boundary and durable implement/defer/reject architecture.
3. `docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md` — portal capability/dependency delivery order; not current ownership or live scheduling evidence.
4. **This programme** — sole live portal selector; resolves current candidates from protected `main`, Issues, tasks, PRs, dependencies and authority.
5. `docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md` — post-selection execution-role and maturity companion only; never a scheduler.
6. The selected Issue/task/branch/PR — exact execution ownership and acceptance for one bounded slice.

`docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json` is a machine-readable **non-scheduling completion-scope projection** of already accepted launch dispositions. It does not select work, claim ownership, prove live state or promote a candidate to `READY`.

## Work allocation

`docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md` is the canonical execution-allocation companion for this programme. It maps accepted portal workstreams to model-agnostic execution roles, optional Codex suitability, dependencies and terminal outcomes **after this programme has selected the live work item**.

The companion is not a second scheduler and must not reorder this programme. Delivery-band labels are non-scheduling metadata. Role assignment never overrides repository authority or live ownership. `IMPLEMENTATION_OWNER` describes bounded responsibility, not a model choice; execution mode is selected separately. A row marked Codex-suitable is **not permission** to invoke Codex, OpenAI API or any owner-funded AI quota. `AGENTS.md` still requires explicit owner authorization for that exact use/task.

## Completion-scope dispositions

The completion-scope projection uses only:

- `REQUIRED` — accepted launch scope must reach a terminal implementation/defer/reject outcome; the selector may not silently skip it;
- `CONDITIONAL` — selectable only when its named accepted activation trigger is proven for the current scope;
- `DEFERRED` — intentionally outside current launch scope until its accepted reactivation trigger is satisfied;
- `REJECTED` — explicitly excluded by accepted authority.

These values are product/release dispositions, **not** selector states. `REQUIRED` does not imply live `READY`, and `CONDITIONAL` does not become active merely because implementation is convenient. Live selector states remain exactly `TERMINAL | OWNED | BLOCKED | DECISION_REQUIRED | READY`.

When the scope projection and a newer accepted ADR/owner decision conflict, the higher authority wins and the projection must be reconciled before it is used for a global completion claim.

## Per-capability disposition proof

Workstream disposition is not proof that every benchmark capability inside that workstream has an accepted product outcome. Before any global Portal Completion claim, resolve the **named release capability inventory** from `PORTAL_COMPLETENESS_ARCHITECTURE.md`, the focused canonical architecture owners for the included capability families, and accepted owner decisions for that release scope.

For that exact inventory:

1. enumerate every named capability with a stable capability ID; a broad workstream/family row must not replace or collapse its member capabilities;
2. prove exactly one durable live disposition record for every capability;
3. every record must contain `capability_id`, `owner`, `rationale`, `outcome` and `authority_evidence`;
4. `outcome` is exactly `IMPLEMENT | DEFER | REJECT` and must be owner-approved under current repository/product authority;
5. `IMPLEMENT` is product disposition only and does not prove implementation, E2E, CI, production readiness or activation;
6. `DEFER`/`REJECT` must retain owner/rationale/authority evidence and cannot be inferred from inactivity, missing ownership or a broader workstream state;
7. missing, duplicate, conflicting or ambiguous capability-disposition evidence is `DECISION_REQUIRED` and keeps global Portal Completion false.

`OTERYN_PORTAL_COMPLETION_SCOPE.json` carries the machine-readable proof contract, not the live disposition records themselves. The records must come from current durable authority/evidence for the named release. A workstream becoming terminal never substitutes for this per-capability proof.

## Architecture decision boundary

`OTERYN_PLATFORM_ARCHITECTURE_REVIEW` owns discovery and resolution of a **new or superseding durable architecture decision**: new module ownership, durable cross-module direction, new trust boundary, major product architecture alternative, or another question that requires an ADR/backlog decision.

The Portal Completion coordinator may apply, decompose and locally reconcile **already accepted** architecture into bounded implementation/documentation slices. It must not invent a durable decision merely to make a candidate executable.

If a selected capability requires an unresolved durable architecture decision:

- reuse an existing Architecture Review Issue/task when one exists;
- otherwise route the exact obligation through the architecture decision backlog/review programme;
- classify the affected runtime candidate `DECISION_REQUIRED` or `BLOCKED` as appropriate until accepted authority exists;
- a bounded architecture/documentation package may itself be selected only when its scope is authorized and it does not claim runtime delivery.

## Live-state rule

This record intentionally does not pin a current branch, PR, Issue claim or `main` SHA. On every invocation the agent must resolve:

- current protected `main` identity;
- active tasks and ownership;
- open and recently relevant Issues/PRs;
- deterministic repair branches;
- reviews, required checks and exact heads;
- merged/superseded work;
- current environment/permission boundaries.

Durable live state overrides every dated example, report, maturity row and convenience index. `docs/agents/ACTIVE_WORK.md` and `docs/agents/PROJECT_STATE.md` are routing aids only: when they conflict with a newer active task, live PR/Issue state or protected `main`, the live state wins and the stale routing text must not participate in selection.

## Canonical selector states

Every exact candidate is classified from live evidence as exactly one of:

- `TERMINAL` — the required current candidate scope is complete, explicitly deferred/rejected by accepted authority, a conditional activation trigger is proven false for current scope, or no resumable candidate remains;
- `OWNED` — a valid live task/branch/PR/lease already owns the candidate; do not duplicate it;
- `BLOCKED` — an exact dependency, authority, environment or required evidence prevents safe execution;
- `DECISION_REQUIRED` — one explicit owner/product/architecture decision is required before execution can be authorized;
- `READY` — the exact candidate satisfies every eligibility rule below and is unowned.

Classification is per live invocation, not a persistent workstream status. For every skipped earlier entry, persist the exact candidate evidence/reason in the task or selection proof.

### Mixed-entry candidate classification and roll-up

A numbered selection entry may contain more than one exact candidate (for example LiveOps and PublicPortal Today, or Wiki and Game Catalog). The selector must never collapse those candidates before checking whether an unowned `READY` sibling exists.

For each numbered entry:

1. enumerate every exact currently relevant candidate package that can be identified from accepted architecture/programmes, the completion-scope trigger where applicable, live Issues/tasks/PRs and current dependencies;
2. classify **each candidate** independently as `TERMINAL | OWNED | BLOCKED | DECISION_REQUIRED | READY` with exact evidence;
3. order candidates by an accepted sub-order when one exists; otherwise use dependency order first, then accepted programme/delivery sub-priority, then canonical candidate identity (Issue number ascending, otherwise task/package identifier lexical ascending) only as a final deterministic tie-breaker among otherwise independent peers;
4. roll the entry up with this strict precedence: `READY` if any candidate is `READY`; otherwise `OWNED` if any candidate is `OWNED`; otherwise `DECISION_REQUIRED` if any candidate is `DECISION_REQUIRED`; otherwise `BLOCKED` if any candidate is `BLOCKED`; otherwise `TERMINAL`;
5. when the entry rolls up `READY`, select its first `READY` candidate in the candidate order above and stop traversal before considering any later numbered entry.

`READY` therefore outranks `OWNED` and `BLOCKED` at entry roll-up specifically so an owned or blocked sibling cannot make an independent ready candidate unreachable. The proof must still retain all candidate classifications; roll-up is only the entry-level selector state.

## Selection order

Select the first **safe, unowned, unblocked and implementation-authorized** item that remains unresolved:

1. Resume the current valid portal-completion task when one exists and ownership permits it; otherwise classify the entry `OWNED` or `TERMINAL` with evidence.
2. Reconcile **current portal-routing** source-of-truth/task/PR/selector drift when it materially prevents reliable selection or closeout. Historical `RETAIN`/`RECOVERY`, historical-ref preservation/deletion and steady-state branch hygiene are repository-governance concerns under ADR 0037/0039 and the Historical Branch Audit, not Portal Completion P0 candidates.
3. Route any **currently open, implementation-authorized, high-risk** repair Issue through `OTERYN_PLATFORM_REMEDIATION`. Historical examples #948, #944 and #941 are terminal and must never be treated as current queue entries merely because they remain referenced in history.
4. Resolve production/public-edge evidence only with explicit protected-environment authority and access.
5. Complete core account/character lifecycle and ADR 0030 Character Portfolio Platform slices.
6. Deliver LiveOps and public Today; add private Today only after identity/privacy/cache gates and only when its current completion-scope trigger is active. Focused architecture may be selected independently of a blocked runtime producer, but runtime current-state facts require exact authoritative producer evidence.
7. Remove the Announcements/Events reverse edge and deliver federated search when its current accepted scope disposition/trigger permits it.
8. Deliver the accepted first-party Client Distribution Platform boundary, including Issue #1039 or its live successor when current accepted scope requires it, while keeping external updater implementation, protected signing and production activation behind their separate gates.
9. Close Wiki/Game Catalog expected-content inventories required by current scope.
10. Deliver individually promoted PlayerCompanion follow-up vertical slices. The Hunt Session Analyzer v1/foundation is terminal; follow-ups are independent candidates and are not implicitly launch-required merely because they exist in the architecture backlog.
11. Deliver World Hub/community expansions only when their accepted conditional/deferred trigger is promoted and authoritative inputs/product need exist.
12. Activate commerce only after its independent product disposition, security, legal, operational and owner-authorization gates.

Use the work-allocation companion only after the live item has been selected. The companion may choose the execution role and permitted execution mode for that selected item, but it must not select a later matrix row merely because it is easier, Codex-suitable or already decomposed.

## Cross-cutting multi-world / ruleset / season invariant

`world`, `profile`, `ruleset`, `catalog snapshot`, `season` and effective-period dimensions are a **conditional cross-cutting invariant**, not an independent standing queue item.

For every selected slice, verify whether its URLs, identities, cache keys, projections, events, formulas or persistence can preserve the applicable dimensions without an irreversible single-world/global assumption.

- If accepted architecture already defines the needed dimensions, implement them inside the selected slice.
- If a slice would create a new unresolved durable dimension/identity/compatibility decision, route that exact decision through Architecture Review before runtime implementation and classify the runtime candidate accordingly.
- Do not create speculative multi-world infrastructure solely because the cross-cutting invariant exists.

## Eligibility

An item is `READY` only when all of the following are proven from the current trusted state:

- one exact candidate Issue/task package exists or can be created under the new-product-slice rule with evidence-backed problem and acceptance inventory;
- its completion-scope disposition is `REQUIRED`, or it is `CONDITIONAL` with the exact activation trigger proven; `DEFERRED`/`REJECTED` work is not silently reactivated;
- implementation/documentation for that exact package is authorized;
- no valid task/branch/PR/lease already owns the same intent or paths;
- every required predecessor decision/ADR/contract is accepted;
- every authoritative source required by the capability is proven without inventing external producer behavior;
- all intended paths and evidence collection fit Platform repository authority;
- production/protected/external mutations are not implied by completion of the package;
- the multi-world/ruleset/season invariant is satisfied or its exact durable decision is separately routed;
- a complete bounded vertical slice is achievable, or the task is explicitly architecture/documentation-only and does not claim runtime delivery;
- rollback and required focused/integration/E2E/final-CI validation are known.

`ARCHITECTURE_READY` in Work Allocation is **not** canonical `READY`. It means only that an accepted architecture boundary exists. Promotion to canonical `READY` occurs transiently only after every live eligibility condition above is proven. Conversely an `OPEN`, `CONDITIONAL` or `ARCHITECTURE_READY` workstream may classify `BLOCKED`, `DECISION_REQUIRED`, `OWNED` or `TERMINAL` in the selector.

## LiveOps promotion invariant

Focused LiveOps architecture is owned by `docs/architecture/LIVEOPS_ARCHITECTURE.md`; architecture Issue #1046 is historical terminal evidence, not an active task.

The first runtime package `WorldStatus + configured Maintenance` becomes canonical `READY` only after the exact authoritative runtime-status source needed by the delivered WorldStatus capability is proven from allowed Platform-side evidence. Platform maintenance intent is separate policy authority and cannot manufacture observed runtime health. `ServerSave` remains unavailable/blocked until its own authoritative producer, applicability, time-base, recurrence and freshness semantics are proven. Missing evidence must never become `offline`, `0`, `none`, success or an invented schedule.

`MODULE_CATALOG.md` therefore remains truthful with `LiveOps | PLANNED` until an executable LiveOps capability is merged and validated; architecture completion alone does not promote module implementation availability.

## Routing rules

- Existing audit repair Issue: use the deterministic Issue claim and `OTERYN_PLATFORM_REMEDIATION` contract.
- New durable architecture decision: route through `OTERYN_PLATFORM_ARCHITECTURE_REVIEW` / the architecture decision backlog before runtime implementation.
- New product slice inside accepted architecture: create or reuse one canonical Issue, one task record, one branch and one PR under current governance.
- Existing authoritative PR: reuse/fix/rebase it when safe; do not create a duplicate.
- Superseded/duplicate/obsolete PR: close only with concrete proof and preserve unique work/evidence.
- Server/game dependency: mark the exact candidate `BLOCKED` or `DECISION_REQUIRED` as appropriate; do not inspect an external repository without separate owner permission.
- Production dependency: perform only repository-safe work, persist the exact operator gate and select another independent safe item when possible.
- Historical branch/ref concern: route to repository branch-lifecycle/hygiene governance; do not make it a portal feature candidate.
- Specialized programme dependency: route detailed decomposition through that programme without changing this selection order; for Game Catalog this includes `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md`, whose planning record cannot grant external-repository authority.

## Parallelism contract

- One selected candidate has one Issue/task/branch/PR owner.
- One worker/invocation entry selects at most one new candidate before execution.
- Multiple already-owned, dependency-independent portal tasks may run globally in parallel when their `owned_paths` do not overlap and no ordering dependency exists.
- An `OWNED` candidate is evidence to avoid duplicate work, not a reason to steal or join another worker's branch.
- Never run multiple workers inside the same active PR/worktree.

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
- source-branch/resource closeout is verified;
- the programme queue is re-evaluated from the new protected `main`.

Global Portal Completion may be claimed only when all of the following are proven from current durable evidence for the exact named release scope:

- every `REQUIRED` scope item has a terminal implementation/defer/reject disposition;
- every active `CONDITIONAL` trigger has a terminal disposition;
- the exact canonical per-capability inventory has been resolved from current architecture/owner authority and every capability has exactly one owner-approved `IMPLEMENT | DEFER | REJECT` record containing stable `capability_id`, `owner`, `rationale`, `outcome` and `authority_evidence`;
- no broad workstream/family disposition is being used as a substitute for the required per-capability records;
- missing, duplicate, conflicting or ambiguous capability-disposition evidence is absent; otherwise the programme remains `DECISION_REQUIRED` and global completion is false;
- no launch-critical repair remains unresolved;
- any production/go-live claim is backed by separately authorized direct environment evidence for the exact deployed identity.

The scope manifest and workstream terminality never substitute for the live per-capability proof or the other live facts above.

## Real stop conditions

Return a terminal status only for:

- `DONE`: the selected item and required closeout are terminal;
- `WAITING`: a genuine accepted external actor, permission/environment, protected operation, observation window or bounded terminal-CI wait;
- `BLOCKED`: no safe progress exists because an exact decision, authority, evidence or dependency is missing;
- `ROTATE`: durable next action requires another eligible session/task boundary under repository execution-budget rules.

Do not use `WAITING` for another internal worker, reviewer, auditor or hypothetical future task.
