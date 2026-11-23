# GAP-55 · Party recruitment qualification

The Platform owns the recruitment board, listing lifecycle and report queue. Enable
`PORTAL_PARTY_ENABLED=true` in a qualified environment to expose **Community →
Find a party** in Polish and English. The switch defaults to off. Posting and
reporting are browser actions. The listing gives the qualified character name
for contact in Game.

| Actor | Platform surface |
| --- | --- |
| Visitor | `/{lang}/party` filters active, qualified listings by world, purpose, eligible level and vocation. |
| Eligible player | The board offers characters with current ownership and matching profiles. **Your listings** shows active, removed and expired posts, and lets the owner withdraw an active post. Listing times are UTC. |
| Support moderator | The board links authorized operators to `/admin/{lang}/party` on the configured staff host. The queue shows open reports, listing state and independent ownership/profile feed freshness. The staff guard requires MFA, `admin.access` and `support.reports.manage`. Each decision resolves the displayed batch and records an audit entry in the same transaction. |

Posting requires a description with 1–500 characters, an ordered level range,
a start at or after the database clock and expiry after the start, within seven
days. Expiry removes a listing from discovery and closes further reporting.
Owner and staff views derive expiry from the database clock even if no process
has advanced the stored lifecycle state. The existing per-account posting and
reporting limits apply.

## Game qualification before enablement

- Qualify the LCFA producer and its watermark in the target environment, with
  the committed creation/transfer receipt chain or an accepted ownership baseline
  for the characters used in qualification.
- Qualify the public `character_profile` producer and keep its watermark current.
  The name and world must agree with the ownership projection, and the profile
  must carry a supported vocation and level.
- Demonstrate the two feeds together for a real character. Post, find by filter,
  and check expiry. Verify foreign ownership refusal and the unavailable state
  when either feed falls behind. The operator's feed health is a freshness
  diagnostic. Each character still requires its own proof.

All recruitment mutations write Platform records. Missing or conflicting
authority leaves posting unavailable. Test fixtures qualify Platform behavior
only. The browser spec uses Platform history to exercise withdrawal and
moderation, and the Rust authority test supplies isolated projection data to
exercise publication and discovery.

Contract references

- [Character authority commands](../contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md)
- [Public Game data projection](../contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md)
