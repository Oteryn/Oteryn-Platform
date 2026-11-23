# GAP-52 incident dossiers

Staff with `admin.access`, `audit.view`, and verified MFA can open **Incident dossiers** from Administration. The index lists their assignments and looks up a case by UUID. Staff with `admin.roles.manage` can also open cases and change the assigned investigator. The investigator must have an active account and current staff and audit grants. A reassignment removes the previous investigator’s access. A case manager can record an assignment and receive its case ID. Reading the dossier requires that manager to be the assigned investigator.

## Platform workflow

1. Open a case with the investigator’s account UUID and the exact ADM target. The world is required. Leave node, character, and Platform account empty when those fields are null in the ADM target. Optionally attach the UUID of a binding conflict alarm from the existing console.
2. Use the generated case UUID as the ADM reason’s `ticket_ref` where a supported workflow already accepts a case reference. The dossier matches the reference and all four target fields exactly. Requests with other references or targets stay outside the case. Older ADM records retain their original references.
3. Open the assigned case and set a date window. Dates are inclusive UTC days, with the most recent 30 days selected initially. Select up to 31 days at a time. Each page shows up to 100 events and links to older events in the same window when available.
4. Inspect case and assignment events, the linked alarm’s time and category, ADM request and dry-run metadata, independent approvals, and recorded transitions. Request and approval IDs connect the preview and subsequent execution when the corresponding records exist. The dossier commits a read audit with the window and redacted row count before returning the timeline.

The projection exposes only selected identifiers, event types, timestamps and stored status. Alarm details, request bodies, reasons, raw previews, receipt payloads and Game ledger references stay outside the dossier. Game evidence, approval for Game effects and verified Game outcomes display **Unavailable**. Existing ADM Game-effect controls are default off. The dossier creates Platform case records and audit events.

## Game qualification required

The proposed private incident evidence profile requires the Game evidence producer and family allowlist, case and subject scope verification, retention and disclosure policy, stable cut and cursor binding, watermarks and gap reporting, and versioned evidence suitable for verified ingestion. Each Game domain owns its source facts. [Private incident evidence profile](../../../../../docs/contracts/PANEL_PROPOSED_GAME_OPERATIONS.md#8-private-incident-evidence-timeline)

For effect approval and an authoritative outcome, Game and Platform must ratify the typed dry run and preview digest, current requester and independent approver scope, authorization and revocation fences, trusted signatures, correlated receipts, and reconciliation across owner changes. Before enabling the affected controls, qualify these checks end to end and wire the scoped read model and approval review to the accepted evidence. [Proposed Game operations](../../../../../docs/contracts/PANEL_PROPOSED_GAME_OPERATIONS.md)
