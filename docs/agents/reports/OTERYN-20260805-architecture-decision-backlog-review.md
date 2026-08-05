# Architecture decision backlog authority review

## Review identity

- Programme: `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`
- Backlog item: `ARCH-AUTH-005`
- Issue: #602
- Pull request: #604
- Exact base: `main@aa3ddcd0513708276920cb2734f7be845c3f177a`
- Review date: 2026-08-05
- Owner decision: **Option B accepted**
- Accepted ADR: `docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md`
- Runtime E2E: `NOT_APPLICABLE` — this package changes architecture decision documentation only.

## Current state

### PROVEN

- ADR 0022 places accepted ADRs above programme, task, Issue and PR records in architecture authority order.
- The architecture programme embeds a compact `decision_backlog`, but no independent schema or validator owns it.
- `tools/validation/adr_registry.py` already provides fail-closed ADR identity and lifecycle validation.
- Repository-file, Issue and PR searches found no separate machine-readable architecture decision backlog or equivalent owner.
- GitHub Issues provide live collaboration state but are not reproducible exact-head repository artifacts.
- ADR prefix 0023 was free after scanning the ADR inventory and open architecture PRs.
- The repository owner accepted Option B in the current invocation and the decision was recorded on Issue #602.

### DERIVED

- Keeping the full backlog in programme state would make one file both continuation state and durable decision inventory.
- Using GitHub Issues alone would make exact-head validation depend on mutable remote state and API availability.
- A dedicated repository registry remains subordinate to accepted ADRs because it is restricted to unresolved decision obligations.

### UNKNOWN

- Exact implementation-package final head, CI conclusions and merge commit.
- Whether future live reconciliation will be a separate command or an optional validator mode; the deterministic offline boundary is already fixed.

### CONFLICT

None.

## Accepted decision

The repository adopts **Option B — dedicated canonical JSON backlog plus deterministic validator**.

Confidence: **high**.

The accepted authority boundary is:

- accepted ADRs remain durable architecture decision authority;
- focused canonical architecture documents remain current concern owners;
- `docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json` contains unresolved, blocked or intentionally deferred decision obligations only;
- GitHub Issues own discussion, assignment and explicit owner choice;
- the architecture programme stores only a compact active-ID projection and one `next_action`;
- implementation is a separate bounded package under the remediation lifecycle.

## Rejected alternatives

### Programme-embedded full registry

Rejected because it mixes execution state with a durable inventory, increases merge contention and couples schema evolution to programme prose.

### GitHub Issues and labels as the sole backlog

Rejected because it cannot provide reproducible exact-head offline validation and has weaker structural guarantees.

### Status quo

Rejected because lifecycle, deduplication, evidence and authority boundaries remain unenforced.

## Accepted record lifecycle

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

## Validation boundary

Local deterministic validation covers schema, IDs, evidence-state separation, lifecycle, local path/ADR references, duplicate obligations, programme projection and non-authority invariants.

Live GitHub reconciliation covers Issue/PR open/closed/merged state only when a transition depends on remote state. Unavailable remote state produces `UNKNOWN` or fails closed; it is never guessed.

## Separate implementation handoff

One bounded implementation package must:

- add `docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json`;
- add validator/tests under `tools/validation/**`;
- execute validation through the existing test suite without workflow edits where possible;
- seed only unresolved records;
- shrink programme state to active IDs and one next action;
- update architecture authority routing narrowly;
- archive the implementation task and release ownership after exact-head validation.

No application, migration, dependency, deployment, infrastructure, production or external-repository work is authorized by this decision.

## Review outcome

The design decision is complete and accepted. PR #604 must be validated on its final exact head, merged, and the design task archived before the separate implementation package claims its non-overlapping paths.