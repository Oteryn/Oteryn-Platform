# Collection battles and fixed sets

The existing localized `GET|POST /{lang}/collection-pvp` handles challenges, turns,
round-robin tournaments, rankings, deck management and fixed set purchases.
Players reach it through Play navigation. Sessions, CSRF and value eligibility
apply to form submissions. Account identifiers can be shared with another player
to receive a challenge.

## Activation

- `OTERYN_COLLECTION_PVP_ENABLED=true` enables Platform battles and tournaments.
- `OTERYN_COLLECTION_SETS_ENABLED=true` enables the fixed set catalog and coin
  purchases. The wallet debit is classified as `card_set_purchase`.
- Both switches default to off. Card admission needs enabled World reads and
  published, visible T90 card definitions for the active World version.

Publish offers through the existing trusted `collection_pvp_sets` catalog. The
SKU is the offer ID. Its version, coin price and card quantities are immutable
and the offer must be published. Each content key is the canonical UUID of a
published `collection_cards` row. Retire an offer by clearing its published flag.
Create a new SKU to change its contents or price.

The wallet operation, audit entry and immutable purchase receipt commit together.
The receipt is the account's entitlement to the exact published card quantities.
An exact retry returns the existing receipt. A further purchase of the same SKU
by that account is refused. Purchases can stay off while active, visible cards
the account already owns remain usable for PvP.

## Match and tournament rules

Players select one to three distinct owned cards in one of up to five saved
decks. A challenge pins the initiator's deck and requires the recipient's
acceptance. Tournament registration pins each entrant's deck and consents to
each round-robin fixture. The organizer starts the tournament after at least two
entrants join. Capacity is limited to 16. Tournaments have no fee, stake or prize.

Each participant submits one private strike or guard per round. The server
resolves both moves together using the versioned collection combat rules and
private RNG state. Revisions and row locks serialize moves and score each match
once. Standings award three points for a win, one for a draw and zero for a loss.
The participant view shows health and submission status. It omits opponent
choices and RNG state.

## Game qualification

The live startup can use published cards acquired through the Platform wallet.
The bestiary deck path shows the existing unavailable state until the T90
journal authority and catalogue mapping are qualified. When supplied, the T90
adapter checks current ownership, fresh evidence and card publication inside
the caller's transaction before a deck or turn is admitted. Game still needs a
qualified G-11 bestiary publication and accepted creature/race/appearance mapping
for this path. Battles, tournaments, purchases and grants issue no Game writes.

## Validation

The minigames library tests exercise the real Wallet and published-card adapters
against a disposable schema, including an insufficient balance, invalid
contents, an exact retry, a duplicate SKU, a foreign deck and a completed
tournament. The `gap-62.spec.mjs` browser spec uses the local runner, the sandbox
wallet top-up and a disposable published card and SKU.
