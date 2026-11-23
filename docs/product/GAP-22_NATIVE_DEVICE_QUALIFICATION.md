# Native remembered-device qualification

In the portal, open Account → Native remembered devices. The page shows the
account's remembered families, their enrollment identifiers and lifecycle state.
To revoke a family, confirm the account password and then select Revoke device.
The existing sudo session lasts up to 15 minutes. The Platform records the
revocation and expires unused Platform tickets linked to that family in one
transaction. A revoked family is retained for review. Game session state appears
as unavailable until there is evidence from Game. The page remains usable for
revocation of persisted families while native enrollment is disabled.

## Contract status

| Document | Status | Use |
| --- | --- | --- |
| [Original candidate](../contracts/NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT_CANDIDATE.md) | Historical | Design history for the separate native credential. |
| [Native device session service](../contracts/OTERYN_V2_NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT.md) | Accepted for the service | Protocol 1 transport and wire behavior, enrollment proof, session classes, and rotation and revocation. |
| [Native remembered-device client](../contracts/OTERYN_V2_NATIVE_REMEMBERED_DEVICE_CLIENT_CONTRACT.md) | Proposed, with superseded wire details | Client qualification checklist. Use the accepted service contract for request and response bodies. |
| [Gateway login](../contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md) | Governs protocol 3 recovery | Device binding and recovery release gates in §19.15. |

## Operator status

The account page reports whether native enrollment is disabled, unavailable,
or configured for qualification. It checks the deployment selectors, TLS
configuration and whether an allowlisted native OAuth client is registered.
Game qualification appears as unavailable. The existing
`OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED` switch defaults to off. The enrollment
routes retain the testing/preproduction restriction, direct TLS listener,
OAuth allowlist and explicit native client registration requirements in the
accepted service contract. The status is a configuration readout. Prove client
and Game readiness through the sequence below before any rollout decision.

## Remaining client and Game evidence

1. In an isolated testing or preproduction deployment, use the real client and
   its external browser OAuth flow. Verify explicit opt-in and bound enrollment.
   Exercise OS-vault custody, restart, cross-process rotation and removal of
   local credentials after sign-out or an uncertain rotation.
2. Verify the enrolled family appears in the signed-in owner's browser. Check
   that a different account cannot use or revoke that family and that the
   service refuses a well-formed but invalid enrollment proof.
3. Issue an unused ticket, revoke the device from the browser, and exercise
   rotation and ticket redemption through the real client and Game gateway.
   Verify refusal and check already-admitted sessions against the Game
   session lifecycle contract.
4. Capture direct TLS, independent security review and the §19.15 recovery
   release evidence. Keep the recovery restart question U23 open until the
   owning contract resolves it.

The portal browser spec uses disposable, persisted Platform families to
exercise the account flow. It supplies no client, vault or Game qualification.
