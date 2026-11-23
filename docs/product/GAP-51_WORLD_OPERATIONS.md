# GAP-51 · World operations

Staff can open **World operations** from **Administration**. Select a world and
action, enter a UTC window and a change ticket, then submit the plan. Use a
window beginning within 90 days and lasting up to 24 hours. A scoped reviewer
can open the immutable submission and approve or decline it before its start.
Requests awaiting review expire at the start. Approved windows expire at the
end. The detail page shows the decision, its actor and time, and the separate
Game evidence status.

The maintenance actions cover entering and exiting maintenance, draining
admission, and restarting a named node. The event actions cover starting,
stopping and cancelling an instance. A broadcast plan records the proposed
world or channel audience, language and plain text. Target IDs and content in
these plans are staff proposals. The Game preview, verified revisions and
results display the portal's **Unavailable** state.

## Platform authorization

The configured staff host, staff MFA, and `admin.access` apply. Operations
access requires an explicit `world_operation_planning_grants` row per actor,
world and action. The grant covers viewing and proposing within its scope.
`can_review` permits a distinct actor to review in that same scope. Both
actors' grants and active staff accounts are checked again at review. Creation
and review write audit records in the same transaction as the plan change.
The plan keeps the actor and world IDs for audit correlation.

There are no seeded grants. Under the Platform database change procedure, an
authorized database operator can bind `$1` to the approved staff account UUID,
`$2` to a world registry UUID and `$3` to a planning action key. For
review access, bind `$4` to true only when explicitly approved.

```sql
INSERT INTO world_operation_planning_grants (account_id, world_id, action, can_review)
VALUES ($1, $2, $3, $4)
ON CONFLICT (account_id, world_id, action)
DO UPDATE SET can_review = EXCLUDED.can_review;
```

To revoke the grant, delete the row matching those first three parameters.
The action keys are `enter-maintenance`, `exit-maintenance`, `drain`,
`restart`, `event-start`, `event-stop`, `event-cancel` and `broadcast`.
Inherited staff roles supply the staff boundary. The planning grant table
supplies the world and action boundary.

## Game qualification and Platform follow-on

The proposed contracts are in [Panel Game operations §§9–11](../contracts/PANEL_PROPOSED_GAME_OPERATIONS.md#9-in-world-broadcast).
Current Platform approvals only record agreement on a proposed window.
Qualification needs the following Game-owned evidence and behavior.

- **World Runtime Operations** must ratify signed previews binding exact
  targets, ordered stages and deadlines, both actors' needed stage grants, the
  runtime revision and owner epoch, expected build, and readiness reference.
  The fenced supervisor must verify current evidence and authorization per
  stage, return a separate job admission and durable ordered receipts, and
  stop at the last safe state on a failed prerequisite or deadline. A drain
  requires quiescence evidence, restart requires matching drain and save
  evidence, and exit requires current readiness.
- **Event Scheduler and Runtime** must supply the event definition and live
  instance revisions, resolved scope and impact, the destructive cleanup
  manifest where applicable, the exact signed preview and transition or
  deferred stage receipts. It must refuse stale revisions and unsafe or
  ambiguous effects.
- **World Messaging** must supply audience resolution, topology revision,
  effective risk, exact message preview and durable per-channel receipts.
  It must recheck scope, deadlines and authorization at publication and
  deduplicate dispatch.

Once those contracts are ratified, Platform needs separate, default-off
effect routes using ADM signing, verified Game previews, fresh step-up and
scope-specific effect grants. Bind any required independent approval to the
exact preview and ordered plan, refuse expired approvals and stale evidence,
and drive progress from verified receipts. Existing planning rows contain
neither execution approvals nor a queue for replay.
