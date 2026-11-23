# ADM result qualification

The Platform admin control listener uses the existing default-off `PORTAL_ADMIN_CONTROL_ENABLED` switch. Its v1 delivery allowlist covers the ratified read queries. The Access page under the staff Console shows which connected agents have current result verification authority, and shows the availability of Game operations. Access requires the existing role-management grant and staff MFA.

## Operator setup

- Give each environment its own mTLS trust and ADM signing keys. Configure the result public key and its UTC validity window on the same `PORTAL_ADMIN_CONTROL_AGENTS_JSON` entry as the verified peer identity, agent ID and exact world/node allowlists. Install the matching private key only on the authorized agent.
- Check the Access page after the peer registers. Current result authority describes the configured trust and observed agent scope. Each delivery still requires an advertised read capability and the current owner epoch for its target.
- Keep the signing window valid through acknowledgment, including retries. Rotate the environment's trust and agent key together. If a delivery is superseded, reconcile the original request under the owner-change procedure. A local expiry records a Platform deadline. A delivered request alone establishes no Game outcome.

## Game qualification still required

- Agree on and qualify operation-specific command and preview payloads in a versioned result contract with closed status and error shapes.
- Qualify environment-scoped agent and Platform signing trust, Game route/owner checks, and the current authorization fence at the effect boundary. For approved operations, include the active approval and exact preview binding.
- Have Game durably persist and sign each bound outcome, retain the exact envelope until acknowledgment, and demonstrate retry and owner-change reconciliation without repeating an effect. Enable a command profile only after joint acceptance.

See [the proposed operation profiles](../contracts/PANEL_PROPOSED_GAME_OPERATIONS.md) and [the ratified ADM wire](../contracts/ADMIN_CONTROL_AGENT_WIRE_RATIFICATION.md).
