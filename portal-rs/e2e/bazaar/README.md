# Bazaar legacy item routes (#2002)

The ten rows below come from the [generated unhit ledger](https://github.com/Oteryn/Oteryn-Platform/blob/abe4333225ee3cbd655195e28bbfc794f685d0bc/portal-rs/e2e/ledger/LEDGER.md#L107-L123). Paths are relative to `portal-rs/` and start with `/{lang}/bazaar` in the table.

| Ledger row | Legacy POST | Form source |
| --- | --- | --- |
| F-bazaar-46767873991968 | `/items` | [new.html:1](../../templates/bazaar/new.html#L1) |
| R-bazaar-13638167717685 | `/items` | [new.html:1](../../templates/bazaar/new.html#L1) |
| F-bazaar-23275926065244 | `/items/{id}/renew` | [item.html:6](../../templates/bazaar/item.html#L6) |
| F-bazaar-86148115874390 | `/items/{id}/cancel` | [item.html:6](../../templates/bazaar/item.html#L6) |
| F-bazaar-140015055205774 | `/items/{id}/bid` | [item.html:7](../../templates/bazaar/item.html#L7) |
| F-bazaar-160340630540621 | `/items/{id}/accept` | [item.html:5](../../templates/bazaar/item.html#L5) |
| F-bazaar-187660078956952 | `/items/{id}/reject` | [item.html:5](../../templates/bazaar/item.html#L5) |
| F-bazaar-221617554685096 | `/items/{id}/offer` | [item.html:7](../../templates/bazaar/item.html#L7) |
| F-bazaar-231195288605257 | `/items/{id}/publish` | [item.html:6](../../templates/bazaar/item.html#L6) |
| F-bazaar-250676990346223 | `/items/{id}/buy-now` | [item.html:7](../../templates/bazaar/item.html#L7) |

## Removal candidates for the Bazaar owner

All ten rows share the same disposition. The fallback mounts the old POST paths
and sends a localized 503 before parsing the listing, action, or form. Its
catalogue, new-listing, and detail reads also render the unavailable page, which
has no form or item link. The old catalogue and form templates are reachable
only through the generic item router, which the portal binary never selects.
[main.rs:262](../../bin/oteryn-portal/src/main.rs#L262) · [web.rs:69](../../crates/bazaar/src/items/web.rs#L69) · [web.rs:89](../../crates/bazaar/src/items/web.rs#L89) · [list.html:1](../../templates/bazaar/list.html#L1) · [message.html:1](../../templates/bazaar/message.html#L1)

The Game-backed router mounts GET item catalogue, new-listing, and detail URLs.
Catalogue cards link to the canonical listing URL. The new-listing alias posts
to `/account/bazaar/sell`, and detail renders `/bids` for another player or
`/control` for the seller. The ledger's `{id}` and `{listing}` item detail
patterns are the same concrete GET URL. Legacy create POST has no method
handler and legacy item action POSTs have no route in this composition.
[main.rs:225](../../bin/oteryn-portal/src/main.rs#L225) · [browser.rs:32](../../crates/bazaar/src/game/runtime/browser.rs#L32) · [browser.rs:191](../../crates/bazaar/src/game/runtime/browser.rs#L191)

The specs exercise the mounted fallback refusals and the live
replacement with two accounts. Legacy success, input validation, and ownership
checks are unreachable because the fallback refuses before dispatch and the
Game router serves different mutations. The focused Game-runtime check proves
listing activation, bad input and foreign-owner refusal on control, owner
cancellation, and removal from the public detail view. Existing Bazaar specs
cover funded bids and settlement on the live flow. [legacy.spec.mjs](legacy.spec.mjs) · [bazaar.spec.mjs](../specs/bazaar.spec.mjs) · [game-sim-bazaar.spec.mjs](../specs/game-sim-bazaar.spec.mjs)

## Signed snapshot with items

`F-bazaar-249147474695525` remains open. The existing producer signs the pinned
`locked_character_snapshot` vector with `items: null`. Its seed endpoint accepts
asset and character identities and cannot supply snapshot items. The Game
runtime can render a populated item list, but the ledger's item-search GET
form is in the older character template. Closing this row needs a harness
publisher that provides a signed snapshot with items and a decision about
which browser surface should expose the item search.
[bazaar-game.mjs:77](../support/bazaar-game.mjs#L77) · [bazaar-game.mjs:234](../support/bazaar-game.mjs#L234) · [decisions.json](../../crates/bazaar/tests/fixtures/plat3/decisions.json) · [characters.html:37](../../templates/bazaar/characters.html#L37)

## Run

Use the disposable local PostgreSQL instance and installed Playwright
dependencies described in the [browser harness README](../README.md). From
`portal-rs/e2e`, export `PORTAL_E2E_ADMIN_DATABASE_URL` in both shells, then
start the Game-backed stack with `node local.mjs`. The fallback test starts and stops its own `harness.mjs`
instance on port 18082 with a separate disposable database.

The default Playwright config used by the workflow discovers both Bazaar tests
and includes their results in the normal project and shard totals. The dedicated
config runs the same two tests on their own.

```sh
node run.mjs /absolute/scratch/env.json test --config=bazaar/playwright.config.mjs --project=pl-1440 --project=en-390
```
