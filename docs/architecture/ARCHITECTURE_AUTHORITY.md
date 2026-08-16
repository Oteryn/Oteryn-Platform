# Oteryn Platform Architecture Authority

## Status

Accepted on 2026-08-05 through the repository-owner decision recorded in Issue #548. ADR 0022 records the durable decision.

This document is the canonical entry point for architecture authority and routing in `blakinio/Oteryn-Platform`. It does not duplicate the detailed contents of focused architecture documents.

## Scope

Use this index to determine:

- which source owns an architecture concern;
- how conflicting sources are ordered;
- whether a statement is current, proposed, historical or superseded;
- when a conflict requires an ADR instead of local interpretation.

This document grants no runtime, deployment, production, cross-repository or protocol activation authority.

## Authority order

For an architecture question, apply the narrowest relevant authoritative source in this order:

1. **Repository governance and explicit owner decisions** — repository-scoped instructions and owner decisions define authorization and product intent. Durable architecture decisions must be recorded in an accepted ADR.
2. **Accepted ADRs** — accepted decisions govern their stated scope until explicitly superseded or rejected.
3. **Operation-specific contracts** — contracts under `docs/contracts/**` govern producer/consumer, protocol, data-write and compatibility behavior within their declared scope.
4. **Focused canonical architecture documents** — the documents listed below own the current model for their concern.
5. **Exact implementation and validation evidence** — source code, migrations, configuration and exact-head tests prove what is implemented; they do not silently overrule an accepted decision.
6. **Programme, task, Issue and PR records** — these prove active work, ownership, blockers and proposed changes, but are not canonical architecture after completion unless promoted into the sources above.
7. **Historical planning and superseded records** — context only; they cannot direct new work when a current authoritative source exists.

A lower-ranked source must not silently override a higher-ranked invariant. Record the mismatch as `CONFLICT`, identify the affected owner and resolve it through the relevant contract or ADR.

## Canonical owners

| Concern | Canonical source | Boundary |
|---|---|---|
| Authority, precedence and conflict handling | `docs/architecture/ARCHITECTURE_AUTHORITY.md` | Routes to focused truth; does not repeat domain detail. |
| Durable decisions | `docs/architecture/adr/**` and `docs/architecture/adr/README.md` | Decisions, lifecycle, allocation, machine validation and supersession history. |
| Unresolved architecture decision obligations | `docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json` | Active inventory only; never accepted-decision, implementation or activation authority. |
| Repository licensing, distribution and contribution rights | `LICENSE.md`, `THIRD_PARTY_NOTICES.md`, ADR 0026 and `CONTRIBUTING.md` | Proprietary/no-permission baseline for original Oteryn Platform material; file-specific and third-party notices govern their own scope. |
| System context and topology | `docs/architecture/SYSTEM_ARCHITECTURE.md` | Components, trust boundaries and high-level dependency direction. |
| Native Oteryn-v2 integration and Legacy Canary Compatibility | `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md` and ADR 0031 | Owns the Platform-side native-v2/compatibility split, cross-system authority direction, persistence/admission/projection boundaries and migration invariants; does not own Oteryn-v2 implementation or protocol IDL bytes. |
| Modules and responsibility | `docs/architecture/MODULE_CATALOG.md` | Module ownership, responsibilities and dependency boundaries. |
| Portal completeness, benchmark disposition and release-scope closure | `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md` | Current portal assessment, remaining architectural gaps, implement/defer/reject baseline and portal completion gate. |
| Portal capability/dependency delivery order | `docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md` | Portal-specific implementation/dependency sequence subordinate to accepted architecture; not live ownership or current-candidate scheduling. |
| Portal completion-scope projection | `docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json` | Machine-readable non-scheduling projection of already accepted `REQUIRED | CONDITIONAL | DEFERRED | REJECTED` dispositions; never live state, ownership or `READY` authority. |
| Portal live execution selection | `docs/agents/programs/OTERYN_PORTAL_COMPLETION.md` | Sole live portal candidate selector from protected `main`, Issues, tasks, PRs, dependencies and current authority; it applies architecture but does not supersede it. |
| Portal post-selection execution allocation | `docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md` | Role/maturity mapping after selection only; cannot reorder the live selector or change completion scope. |
| Player calculators, plans, hunt guidance, session analysis and recommendations | `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md` | Focused `PlayerCompanion` ownership, result classification, ruleset/version applicability, privacy, API/client reuse and delivery priorities. |
| Live operational world/service state, maintenance and freshness | `docs/architecture/LIVEOPS_ARCHITECTURE.md` plus `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md` | Owns Platform LiveOps projection/history/public-consumption semantics while preserving separate Platform configured policy and Oteryn-v2 observed runtime authority; does not define producer transport, game-runtime implementation or production activation. |
| PublicPortal Today / command-centre composition | `docs/architecture/PUBLIC_PORTAL_TODAY_ARCHITECTURE.md` and ADR 0032 | Owns Today provider composition, public/private representation, partial-failure, freshness, cache and presentation semantics; source modules retain fact/publication/privacy authority and private/shared-cache isolation cannot be weakened. |
| Federated public content search and discoverability | `docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md` and ADR 0033 | `PublicPortal` orchestration over source-owned public queries; source publication/privacy authority remains upstream and any dedicated index is derived state. |
| First-party client distribution and updater trust | `docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md` and ADR 0035 | Owns Platform release-policy/updater-trust separation, TUF role-separated trust, stable/beta and exact target semantics, minimum/mandatory update policy, withdrawal/revocation/rollback and signing-key custody boundaries; does not own external client implementation, private signing operations or production activation. |
| Production topology evidence and OperationsObservability | `docs/architecture/OPERATIONS_OBSERVABILITY_ARCHITECTURE.md` plus `docs/operations/**` evidence records | Owns repository/staging/production evidence classification, operational signal/recovery ownership, liveness/readiness separation and topology/observability/restore/rollback proof requirements; does not own PublicEdge live controls, business/gameplay truth, protected-environment mutation or production activation authority. |
| Public Internet edge, DNS/TLS and origin-ingress evidence | `docs/architecture/PUBLIC_EDGE_ARCHITECTURE.md` plus `docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md` | Owns provider-neutral DNS/proxy, public TLS, redirect/HSTS, edge abuse/admin controls, tunnel/origin/direct-ingress evidence and fail-closed environment classification. Provider-specific Cloudflare operations remain subordinate evidence/tooling; application auth/security and production activation remain separate authorities. |
| General Platform API activation, adaptation and compatibility | `docs/architecture/PLATFORM_API_ARCHITECTURE.md` and ADR 0036 | General PlatformAPI is explicitly deferred until a named consumer/use case exists; owns the future activation checklist, transport-adapter/versioning/privacy/failure/compatibility invariants and keeps specialized game-auth/internal endpoints outside general API classification. Does not authorize runtime endpoints or production activation. |
| Security | `docs/architecture/SECURITY_ARCHITECTURE.md` | Mandatory security invariants and trust controls. |
| Persistent data | `docs/architecture/DATA_OWNERSHIP.md` | Platform, Canary and shared data ownership/write rules. |
| Validation | `docs/architecture/TEST_STRATEGY.md` | Test layers, evidence expectations and E2E policy. |
| Global delivery order | `docs/architecture/ROADMAP.md` | Platform phases, dependencies and exit gates; not portal-specific live scheduling and not proof of implementation by itself. |
| Repository source-branch lifecycle and historical work | ADR 0037, ADR 0039 and the repository Branch Lifecycle / Historical Branch Audit controls | Branches are execution resources, not archives. Historical retention/deletion and steady-state ref hygiene are repository governance, not product/portal scheduling. |
| Cross-component behavior | `docs/contracts/**` | Exact producer/consumer and compatibility contracts for their scope. |
| Native support/moderation game enforcement | `docs/contracts/OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md` subordinate to ADR 0031 | Platform decision/orchestration semantics and authoritative game result boundary; does not own sanction policy, transport or Oteryn-v2 implementation. |
| Native Game Catalog content ownership | ADR 0034 and `docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md` | Game-domain source authority versus Platform immutable snapshot/projection lifecycle and Legacy Canary Compatibility importers; does not own Oteryn-v2 implementation or exact producer bytes. |
| Non-native structured reference content | ADR 0042 and `docs/contracts/NON_NATIVE_REFERENCE_CONTENT_CONTRACT.md` | Owns provenance-pinned `NON_AUTHORITATIVE_REFERENCE` snapshots, static extraction, source-local identity, reconciliation and explicit Wiki/PlayerCompanion reference consumption; never GameCatalog activation, native/legacy authority, runtime truth or third-party expressive-content publication rights. |
| Current execution state | `docs/agents/PROJECT_STATE.md`, active task and live PR | Current work and validation evidence; not a substitute for an ADR or a programme's canonical live selector. |

Dated benchmark reports under `docs/agents/reports/**` are research evidence. They may justify an ADR or focused architecture update but never become canonical product or implementation authority by themselves.

## Portal delivery control-plane hierarchy

For portal work, distinguish global architecture/delivery intent from live execution selection:

```text
ROADMAP.md
  global Platform phase/risk order
    -> PORTAL_COMPLETENESS_ARCHITECTURE.md
       durable portal completion/release boundary
      -> PORTAL_COMPLETION_DELIVERY_PLAN.md
         portal capability/dependency order
        -> OTERYN_PORTAL_COMPLETION.md
           sole live candidate selector
          -> OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
             post-selection execution role/maturity mapping
            -> exact Issue/task/branch/PR
               one execution owner
```

`OTERYN_PORTAL_COMPLETION_SCOPE.json` sits beside this chain as a non-scheduling projection of accepted launch/completion dispositions. It never selects work or substitutes for live evidence.

A lower layer must not reinterpret an earlier layer to make work convenient. In particular:

- the Roadmap does not mean all later portal product gaps are already closed when a historical engineering phase is marked complete;
- the Delivery Plan does not claim current ownership or `READY` state;
- Work Allocation delivery bands/maturity values do not create a second queue;
- a task/PR cannot redefine architecture, completion scope or repository authority by prose.

## Architecture Review versus delivery coordination

`docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md` owns unresolved **new or superseding durable architecture decisions** and their ADR/backlog lifecycle. Examples include a new module owner, new durable dependency direction, new trust boundary or a product architecture choice with alternatives that outlives one implementation task.

A delivery/programme coordinator may apply and decompose already accepted architecture, reconcile implementation evidence to it, and create bounded implementation handoffs. It must not silently decide a new durable architecture question merely to unblock a feature.

When a selected delivery candidate exposes a new durable architecture obligation:

1. search accepted ADRs/focused architecture and the architecture decision backlog;
2. reuse any existing architecture owner;
3. otherwise route the bounded question through Architecture Review/backlog;
4. keep affected runtime work `DECISION_REQUIRED` or `BLOCKED` until accepted authority exists;
5. after acceptance, return implementation to the normal delivery owner.

## Source-state labels

Architecture statements should be interpreted or labelled as:

- `CURRENT` — the active canonical model for the stated scope;
- `PROPOSED` — under review and not yet authoritative;
- `HISTORICAL` — preserved context that must not direct current work;
- `SUPERSEDED` — replaced by a named decision or document;
- `UNKNOWN` — evidence is insufficient;
- `CONFLICT` — authoritative evidence disagrees and requires resolution.

Implementation availability, staging proof and production proof are separate facts. A roadmap item or accepted design does not prove delivery, and delivered code does not authorize production activation.

## Conflict procedure

When two sources disagree:

1. confirm both sources and their exact scope;
2. classify the fact as `PROVEN`, `DERIVED`, `UNKNOWN` or `CONFLICT`;
3. apply the authority order above without broadening either source;
4. do not invent compatibility or silently rewrite history;
5. update the focused canonical owner and create or supersede an ADR when the resolution outlives one task;
6. preserve links to the displaced historical evidence.

## ADR allocation and validation rule

Before allocating a new ADR, scan all ADR files and open architecture PRs, take the highest numeric prefix, and use the next integer. Do not reuse gaps.

`python3 tools/validation/adr_registry.py` enforces filename shape, lifecycle presence, README inventory equality, supersession targets and numeric-prefix uniqueness. Historical duplicate identifiers are preserved only through the validator's closed exact-path allowlist. Any new collision or change to a legacy path set fails closed.

Existing accepted ADRs must not be renamed or renumbered without a separate compatibility decision.

## Architecture decision backlog routing

`docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json` is the sole repository inventory of unresolved architecture decision obligations. It is subordinate to accepted ADRs and focused canonical documents and must never be treated as proof that a decision is accepted, implemented or activated.

Use the backlog only when a material architecture or repository-governance question remains unresolved after duplicate searches of accepted ADRs, focused canonical documents, live Issues, open PRs and programme state. Do not import ordinary implementation tasks, completed programme history, audit symptoms that already have a repair owner or questions that can be answered directly from an existing authority.

The active lifecycle is:

1. `discovered` — the obligation is proven but evidence or alternatives are incomplete;
2. `analysis_ready` — evidence, alternatives, trade-offs and a recommendation are complete;
3. `decision_required` — one exact owner question blocks resolution;
4. `blocked` — required authority or primary evidence is unavailable;
5. `deferred` — the owner intentionally postponed the decision with a reason and revisit trigger.

Transitions must be recorded in one bounded PR that updates the JSON record, its linked Issue and the compact programme projection as applicable. Ordinary repository validation is offline and does not guess live Issue or PR state. A separate live reconciliation may verify remote state before a transition that depends on it.

A record leaves the active backlog when the obligation is accepted, rejected, proven false or superseded. The same bounded package must record the terminal authority or rationale in an accepted/rejected ADR, focused canonical document, linked Issue or report. Do not retain terminal records in the JSON file and do not create a second permanent decision archive; Git history and linked authority preserve the history.

Run:

```bash
python3 tools/validation/test_architecture_decision_backlog.py
python3 tools/validation/architecture_decision_backlog.py
```

The validator fails closed for schema drift, unsupported lifecycle values, authority claims, evidence duplication, unresolved-reference defects, duplicate obligations, non-canonical serialization and programme-projection drift.

## Change policy

- Update this file only when authority, canonical ownership or conflict handling changes.
- Update focused documents for domain detail.
- Add a new ADR and supersede the old one when a durable decision changes.
- Add, transition or remove decision backlog records only with their linked Issue and exact programme projection.
- Never turn an execution report, chat transcript or PR description into an implicit architecture source of truth.
