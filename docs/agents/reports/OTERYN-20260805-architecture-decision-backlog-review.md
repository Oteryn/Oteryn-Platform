# Architecture decision backlog authority review

## Review identity

- Programme: `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`
- Backlog item: `ARCH-AUTH-005`
- Issue: #602
- Exact base: `main@aa3ddcd0513708276920cb2734f7be845c3f177a`
- Review date: 2026-08-05
- Runtime E2E: `NOT_APPLICABLE` — this package changes architecture decision documentation only.

## Current state

### PROVEN

- ADR 0022 places accepted ADRs above programme, task, Issue and PR records in architecture authority order.
- The architecture programme embeds a compact `decision_backlog`, but no independent schema or validator owns it.
- `tools/validation/adr_registry.py` already provides fail-closed ADR identity and lifecycle validation.
- Repository-file, Issue and PR searches found no separate machine-readable architecture decision backlog or equivalent owner.
- GitHub Issues provide live collaboration state but are not reproducible exact-head repository artifacts.
- ADR prefix 0023 was free after scanning the ADR inventory and open architecture PRs.

### DERIVED

- Keeping the full backlog in programme state would make one file both continuation state and durable decision inventory.
- Using GitHub Issues alone would make exact-head validation depend on mutable remote state and API availability.
- A dedicated repository registry can remain subordinate to accepted ADRs if it is restricted to unresolved decision obligations.

### UNKNOWN

- The repository owner's selected authority model: programme-embedded, dedicated JSON registry or Issues-only.
- Whether future live reconciliation should be a separate command or an optional mode of the deterministic validator.

### CONFLICT

None. The problem is a missing durable decision, not a contradiction among accepted sources.

## Decision invariant

There must be exactly one deterministic inventory of unresolved architecture decision obligations, and it must not become a second authority for accepted architecture.

## Options and trade-offs

| Dimension | A — programme-embedded | B — dedicated JSON | C — Issues-only |
|---|---|---|---|
| Authority clarity | Medium; two roles in one file | High; explicit subordinate registry | Medium; workflow and authority can blur |
| Exact-head reproducibility | High | High | Low |
| Offline validation | Possible but Markdown-coupled | Strong and direct | Not available |
| Merge contention | High on programme file | Low to medium | Low in repository |
| Schema evolution | Awkward | Explicit versioning | Label/API conventions |
| Collaboration | Programme/PR based | Issue plus repository record | Native GitHub |
| Historical snapshot | Git history | Git history | Remote state only |
| ADR duplication risk | Medium | Low with explicit validator rule | Medium |
| Operational complexity | Lowest initially | Moderate | Hidden in remote reconciliation |
| Reversibility | High | High | Medium |

## Recommendation

Choose **B — dedicated canonical JSON backlog plus deterministic validator**.

Confidence: **high**.

It preserves ADR 0022's authority order, keeps the programme compact, supports reproducible exact-head validation and gives unresolved decisions stable identities without granting them accepted status.

## Proposed record lifecycle

Active states:

1. `discovered` — a material decision obligation is proven but analysis is incomplete;
2. `analysis_ready` — alternatives and consequences are complete;
3. `decision_required` — one exact owner question blocks progress;
4. `blocked` — required authority or primary evidence is unavailable;
5. `deferred` — intentionally postponed with owner, reason and revisit trigger.

Terminal handling:

- an accepted decision is removed from the active backlog in the package that accepts or adds the authoritative ADR/canonical update;
- a rejected option or closed false positive is removed in the package that records the rejection rationale;
- superseded obligations are replaced by the new stable decision ID and linked in the Issue/report;
- Git history, Issues and ADRs preserve terminal history; no second permanent decision archive is created.

## Proposed validation boundary

Local deterministic validation should cover schema, IDs, evidence-state separation, lifecycle, local path/ADR references, duplicate obligations, programme projection and non-authority invariants.

Live GitHub reconciliation should cover Issue/PR open/closed/merged state only when a transition depends on remote state. Unavailable remote state must produce `UNKNOWN` or fail closed; it must not be guessed.

## Delivery implications

After owner acceptance, one bounded implementation package should:

- add `docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json`;
- add validator/tests under `tools/validation/**`;
- execute validation through the existing test suite without workflow edits where possible;
- seed only unresolved records;
- shrink programme state to active IDs and one next action;
- update architecture authority routing narrowly;
- archive the implementation task and release ownership after exact-head validation.

No application, migration, dependency, deployment, infrastructure, production or external-repository work is authorized by this review.

## Blocking question

Accept Option B and ADR 0023, or select A/C with the reason that outweighs its stated authority, reproducibility and maintenance costs.