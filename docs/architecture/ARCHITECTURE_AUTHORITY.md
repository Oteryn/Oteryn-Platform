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
| System context and topology | `docs/architecture/SYSTEM_ARCHITECTURE.md` | Components, trust boundaries and high-level dependency direction. |
| Modules and responsibility | `docs/architecture/MODULE_CATALOG.md` | Module ownership, responsibilities and dependency boundaries. |
| Security | `docs/architecture/SECURITY_ARCHITECTURE.md` | Mandatory security invariants and trust controls. |
| Persistent data | `docs/architecture/DATA_OWNERSHIP.md` | Platform, Canary and shared data ownership/write rules. |
| Validation | `docs/architecture/TEST_STRATEGY.md` | Test layers, evidence expectations and E2E policy. |
| Delivery order | `docs/architecture/ROADMAP.md` | Phases, dependencies and exit gates; not proof of implementation by itself. |
| Cross-component behavior | `docs/contracts/**` | Exact producer/consumer and compatibility contracts for their scope. |
| Current execution state | `docs/agents/PROJECT_STATE.md`, active task and live PR | Current work and validation evidence; not a substitute for an ADR. |

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

## Change policy

- Update this file only when authority, canonical ownership or conflict handling changes.
- Update focused documents for domain detail.
- Add a new ADR and supersede the old one when a durable decision changes.
- Never turn an execution report, chat transcript or PR description into an implicit architecture source of truth.
