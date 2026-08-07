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
| Player calculators, plans, hunt guidance, session analysis and recommendations | `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md` | Focused `PlayerCompanion` ownership, result classification, ruleset/version applicability, privacy, API/client reuse and delivery priorities. |
| Security | `docs/architecture/SECURITY_ARCHITECTURE.md` | Mandatory security invariants and trust controls. |
| Persistent data | `docs/architecture/DATA_OWNERSHIP.md` | Platform, Canary and shared data ownership/write rules. |
| Validation | `docs/architecture/TEST_STRATEGY.md` | Test layers, evidence expectations and E2E policy. |
| Delivery order | `docs/architecture/ROADMAP.md` | Phases, dependencies and exit gates; not proof of implementation by itself. |
| Cross-component behavior | `docs/contracts/**` | Exact producer/consumer and compatibility contracts for their scope. |
| Current execution state | `docs/agents/PROJECT_STATE.md`, active task and live PR | Current work and validation evidence; not a substitute for an ADR. |

Dated benchmark reports under `docs/agents/reports/**` are research evidence. They may justify an ADR or focused architecture update but never become canonical product or implementation authority by themselves.

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