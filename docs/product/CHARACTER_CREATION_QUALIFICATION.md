# GAP-01 character creation qualification

## Player journey

Sign in to an active account and open **Account overview → Characters → Create character**. Choose a registered world, sex and vocation, enter a name and submit. The portal validates the form and saves a durable request. Replaying the same submission returns the same operation. A reserved name, a duplicate pending request for the same account and world, or a matching name in the account's fresh roster is refused before a new command is stored. Game owns the authoritative name registry and character limits.

The most recent ten creation requests can be reopened from the Characters section. The request page is private to its account.

| Evidence | What the player sees |
| --- | --- |
| Platform accepted the request and Game has yet to send a receipt | Pending, with a refresh link. Intent expiry alone leaves the outcome unknown. |
| Game returned a rejection | The localized refusal, with another attempt available from the creation page. |
| Game confirmed creation and the roster projection is unavailable | Confirmation of creation and the unavailable roster state. |
| Game confirmed creation and the fresh roster is missing the character | Confirmation of creation while the account list catches up. |
| The fresh account roster contains the character ID from the committed receipt | Confirmation that the character is listed and a link to the roster. |

## Operator status

Open **Operations → Dashboard** on the staff host with an active MFA-authenticated account that has `admin.access` and `game.ops.read`. The Character creation section shows browser issuance and exchange configuration, the pending queue, requests still awaiting receipts after expiry, and the committed and rejected receipts recorded over the last 24 hours. These are Platform observations. Validate producer connectivity and roster freshness with the receipt and projection checks below.

Browser command issuance requires `OTERYN_CHARACTER_CREATE_WEB_ENABLED`, `OTERYN_CHARACTER_COMMANDS_ENABLED` and `OTERYN_CHARACTER_COMMAND_CREATE_ENABLED` to be explicitly true, with a valid `OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY`. Enable the existing `OTERYN_ACCOUNT_OVERVIEW_ENABLED` for the roster. The private V1 exchange requires `PORTAL_CHARACTER_COMMANDS_ENABLED`, the matching `PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY` and a verified, dedicated `PORTAL_CHARACTER_COMMAND_IDENTITIES` binding. The existing composed exchange admits testing and preprod. The roster uses `OTERYN_LCFA_ENABLED`, the matching `OTERYN_LCFA_SOURCE_AUTHORITY` and the existing authenticated LCFA publication path with a live watermark.

The disposable browser spec uses the local harness and its mTLS test identities to pull the exact v2 create intent, submit a v1 receipt, and publish the LCFA snapshot and watermark. It checks the browser result, foreign-owner refusal and rejected names without new command rows. The fixture qualifies Platform composition when executed.

## Game-side release evidence

- Demonstrate that the real Game consumer pulls and replays the v2 create intent, checks account, name, world and character capacity, commits creation once, and publishes the matching durable receipt.
- Demonstrate that Game publishes the resulting account roster snapshot and a fresh LCFA watermark and that a new account sees and can select its first character.
- Qualify both paths together on an approved environment. Production rollout needs an approved configuration and evidence through the production authority boundary.

See the [character authority contract](../contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md) and [browser runner](../../portal-rs/e2e/README.md).
