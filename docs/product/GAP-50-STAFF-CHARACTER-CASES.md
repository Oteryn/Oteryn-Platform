# Staff character cases (GAP-50)

Open **Staff panel → Character cases**, select an operation, and enter an open character support ticket ID. You can also start from the **Character cases** link in the staff ticket view. The ticket supplies the affected Platform account. The requester records the proposed world and operation details, references the case evidence, and assigns a different staff member as reviewer.

| Request | Proposed scope |
| --- | --- |
| Create | World, name, sex and vocation for the ticket account |
| Rename | World, character ID, expected character revision and new name |
| Schedule deletion | World, character ID and expected character revision |
| Cancel deletion or restore | World, character ID, expected character revision and original deletion operation ID |
| Ownership repair | World, character ID, expected ownership revision and intended Platform account, distinct from the account on the ticket |

Use opaque evidence references. The case stores the exact submitted scope and the review note. The Staff panel requires the configured staff host, a verified session with staff MFA, `admin.access` and `support.tickets.manage`. The independent reviewer also needs `support.character-cases.review`, which has no automatic grants. A role administrator can assign it through **Staff panel → Roles**. The queue and detail view are scoped to the requester and the assigned reviewer. The affected account and the proposed destination account cannot review or request their own repair.

The reviewer can accept the case for qualification or return it for more evidence. A returned case preserves the decision. Submit a new case if the proposed scope or evidence changes. The audit log records the submitting and reviewing actors against the case ID. The case retains the proposed scope and note in the restricted view.

**Character Authority status** shows **Unavailable** for Game ownership, preview, result and deletion state where applicable. Platform acceptance is a triage decision. Effect approval requires a separate current authorization tied to a verified Game preview, and the portal keeps Game effects disabled for these staff profiles.

## Game qualification required

- Ratify the staff-specific actors, case and evidence bindings, world scope, permissions, step-up and expiry. Agree on effect approval against the exact preview and current grants, including an independent approver for ownership repair.
- Agree on versioned create, rename, schedule and restore profiles with canonical identity, expected revision, replay and refusal semantics. Keep staff rename independent of the paid owner flow. Agree on authoritative grace, restore and tombstone evidence and the Game-owned finalization policy.
- Ratify an ownership-repair preview and command profile. Establish current ownership and eligibility, refuse ambiguous or stale ownership and revisions, and reconcile conflicting custody, transfer and settlement under the relevant authority fences before any repair. Return the resulting ownership revision and durable receipt.
- Qualify current authority, receipt authentication and strict intent/target correlation. Prove duplicate and timeout recovery, replay, no-effect refusals, and downstream projection reconciliation using the original operation identity. Only after qualification can Platform wire the effect paths behind default-off per-profile flags and show verified outcomes.

[Character Authority contract](../contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md) · [Panel operation proposals](../contracts/PANEL_PROPOSED_GAME_OPERATIONS.md) · [Panels capability matrix](../architecture/PANELS_CAPABILITY_MATRIX.md)
