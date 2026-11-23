# GAP-47 private player journal

Open **My account → Play → Player journal** in Polish or English. Personal goals
belong to the signed-in account. A goal can carry one optional UTC date. The
journal shows one reminder from that date until the owner completes or deletes
the goal. Reminders are in the journal and use the Platform database clock.
Goals and reminders remain usable while Game data is unavailable.

The Game observations panel uses the existing G-11 read flag, configured source,
and family allowlist. Select a character when current ownership can be proven.
Skill history and PvP can show a labeled historical observation for up to 24
hours. Stamina and skull state require a watermark at most ten seconds old.
A missing or ineligible source displays the journal's unavailable state. After
a transfer, the buyer needs observations from their own cut. Login
history opens the existing T80 page and keeps its own default-off read gate.
Game security actions retain their existing boundary.

An operator with `admin.access` and a current staff MFA grant can open **Journal
qualification status** from the journal. It shows configured switches, producer
identity and source presence, pending mapping, and whether a published baseline
has been recorded. A recorded baseline is evidence of publication only. Every
owner read separately verifies current ownership, recovery, family and freshness.

## Before Game-backed activation

- Deliver and qualify the private G-11 producer per family, with a dedicated
  authenticated purpose, contiguous outbox, complete baseline, retry, transfer
  and restore evidence. Verify owner, non-owner, replay and stale outcomes against
  the accepted revision 4 wire. [G-11 contract](../contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md)
- Publish accepted keys, revisions and owner-visible tracks for bestiary, boss,
  quest and achievement data and complete the mapping for death losses/blessings.
  Death recap stays unavailable until that mapping is accepted. The shared T72
  transport owns the existing record and watermark endpoints. V1 permits
  committed losses and blessings and leaves final-seconds actions unavailable.
  [Definition mapping candidate](../contracts/OTERYN_V2_GAME_JOURNAL_DEFINITION_MAPPING_CONTRACT.md)
- Qualify timers, supported receipt scopes and due-event semantics before enabling
  any Game-triggered reminders or forecasts. Premium expiry also needs current commercial
  agreement, and house rent requires an accepted schema. Live email and web push
  require consent and a separately qualified delivery path. Personal journal dates
  have no effect on game state or commercial entitlements.
