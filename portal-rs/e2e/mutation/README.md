# Browser mutation matrix

From `portal-rs/e2e`, regenerate and verify the source inventory.

```sh
python3 mutation/generate.py
python3 mutation/generate.py --check
```

`forms.json` records source POST forms, including Rust-rendered markup, and write-method router registrations. A form's source and ordinal identify it when several forms share a line. An empty `action` submits to the current document. `actionVariants` captures finite template expansions inferred from literals or the form's rendering code. Runtime identifiers remain template expressions. Locale-prefixed routes share a row, while unprefixed aliases retain their own rows. The inventory includes source registrations behind feature gates and compatibility routes. Native and internal protocol receivers and provider webhooks are outside the browser inventory.

Start the disposable PostgreSQL and Portal harness documented in [the E2E guide](../README.md). Run the matrix from the same directory.

```sh
node run.mjs /absolute/scratch/env.json test --config mutation/playwright.config.mjs --retries=0
```

For a non-default build directory, export `PORTAL_E2E_BINARY` with the same binary used by the harness, or supply its `CARGO_TARGET_DIR`. The spec uses the binary's seed CLI for disposable accounts and the existing browser helpers for login and ticket setup. The checkout tests use the harness test provider and verify durable payment and wallet effects independently of the page.

| POST mutation | a | b | c | d | e |
| --- | --- | --- | --- | --- | --- |
| `/{lang}/account/privacy` | Yes | Yes | Anonymous | Yes | Each save advances the revision |
| `/{lang}/support/tickets` | Yes | Yes | Anonymous | Yes | Request-key replay |
| `/{lang}/support/tickets/{id}/reply` | Yes | Yes | Anonymous, foreign player, content editor on another account | Yes | Stale lock version |
| `/{lang}/support/tickets/{id}/status` (close) | Yes | Yes | Anonymous, foreign player, content editor on another account | Yes | Stale lock version |
| `/admin/support/tickets/{id}` (reply and resolve) | Yes, built-in support role | Yes | Anonymous, other player, content editor lacking `support.tickets.manage` | Yes | Stale lock version |
| `/{lang}/shop/coins` | Yes | Yes | Anonymous | Yes | Operation replay |
| `/{lang}/shop/test-checkout/{intent}` (pay) | Yes | Yes | Foreign player | Yes | One event and one credit |
| `/{lang}/shop/test-checkout/{intent}` (reject) | Yes | Yes | Foreign player | Yes | One event and zero credits |
| `/{lang}/shop/test-checkout/{intent}` (cancel) | Yes | Yes | Foreign player | Yes | One event and zero credits |
| `/{lang}/account/characters/new` | Queued, Game outcome pending | Yes | Anonymous | Yes | Submission replay |

The documented unavailable views cover collection deck creation/deletion, battle start/turn/forfeit, season-pass purchase/claim, and daily puzzle answers. The harness has no accepted collection ownership/progress adapter, season-pass authority adapters, or daily puzzle catalogue. The character creation matrix verifies the pending command and submission replay. [The Game-data spec](../specs/game-data.spec.mjs) uses `harness.mjs --game-data` to publish a CreateCharacter receipt and verify the confirmed outcome.

The remaining inventory rows need scenario fixtures and assertions. The table describes the cases implemented in this spec.

## Runtime dependencies for Game-backed write rows

Here a is an authorized success with a durable effect, b is CSRF rejection, c is
authorization rejection, d is field validation, and e is replay or concurrency.
An unavailable GET establishes why the browser has no valid form context. Columns
b, c and d require their own write requests where middleware can be exercised.

| Inventory write row | Positive and replay dependency |
| --- | --- |
| `POST /{lang}/collection/{character}/decks` | The composed collection router in `crates/minigames/src/collection/http.rs` uses `Ownership(Unavailable)` and a progress store with no configured source. LCFA character projection does not supply collection ownership. |
| `POST /{lang}/collection/decks/{id}/delete` | Requires a saved deck associated with a verified character. The collection composition cannot create that deck through the browser. |
| `POST /{lang}/collection/decks/{id}/battle` | Requires a saved deck and current private collection progress. Both depend on the unavailable collection adapters. |
| `POST /{lang}/collection/battles/{id}` | Requires an active battle and a turn version issued by its start operation. The prerequisites for starting a battle are unavailable. |
| `POST /{lang}/collection/battles/{id}/forfeit` | Requires a real battle owned by the actor. The composed collection router cannot start it. |
| `POST /{lang}/pass/{season}/buy` | `crates/minigames/src/pass/web.rs::configured_router` supplies unavailable authority and payment adapters. The browser cannot resolve a purchasable season or settle a purchase. |
| `POST /{lang}/pass/{season}/claim` | The same router supplies unavailable authority and reward adapters. The browser cannot establish claim eligibility and issue the reward. |
| `POST /{lang}/puzzles/{id}/answer` | `Puzzles::publish` is a Rust service method with no composed ingress or operator CLI. The harness cannot publish an eligible daily definition to supply the puzzle ID. |
| `POST /{lang}/shop` | `crates/store/src/web_backend.rs::configured` constructs the browser backend with `purchases: None` and an access adapter whose purchase target check always returns false. The internal Store delivery protocol cannot supply the browser purchase adapter. |
