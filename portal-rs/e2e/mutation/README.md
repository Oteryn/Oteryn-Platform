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

| Browser mutation | a | b | c | d | e |
| --- | --- | --- | --- | --- | --- |
| `/{lang}/account/privacy` | Yes | Yes | Anonymous | Yes | Each save advances the revision |
| `/{lang}/support/tickets` | Yes | Yes | Anonymous | Yes | Request-key replay |
| `/{lang}/support/tickets/{id}/reply` | Yes | Yes | Foreign player | Yes | Stale lock version |
| `/{lang}/support/tickets/{id}/status` (POST and PUT, close and reopen) | Yes | Yes | Foreign player | Yes | Stale lock version |
| `/{lang}/admin/support/tickets/{id}/reply` (public reply) | Yes | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/admin/support/tickets/{id}/status` (POST resolve and PUT reopen) | Yes | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/support/tickets/{id}/read` | Yes | Yes | Foreign player and content editor | Yes | Unread flag clears once |
| `/{lang}/support/reports` | Yes | Yes | Anonymous, and report stays private from another player | Yes | Request-key replay |
| `/{lang}/shop/coins` | Yes | Yes | Anonymous | Yes | Operation replay |
| `/{lang}/shop/test-checkout/{intent}` (pay) | Yes | Yes | Foreign player | Yes | One event and one credit |
| `/{lang}/shop/test-checkout/{intent}` (reject) | Yes | Yes | Foreign player | Yes | One event and zero credits |
| `/{lang}/shop/test-checkout/{intent}` (cancel) | Yes | Yes | Foreign player | Yes | One event and zero credits |
| `/{lang}/account/characters/new` | Queued, and publisher confirms creation | Yes | Anonymous | Yes | Submission replay |

The documented unavailable views cover collection deck creation/deletion, battle start/turn/forfeit, season-pass purchase/claim, daily puzzle answers, and collection PvP and set purchases. The harness has no accepted collection ownership/progress adapter, season-pass authority adapters, daily puzzle catalogue, or the collection PvP gameplay and wallet adapters. The standard matrix proves that the character creation form persists one pending command. The separate Game publisher run proves the terminal receipt and visible creation result.

The remaining inventory rows need scenario fixtures and assertions. The table describes the cases implemented in this spec.

For the owner and staff ticket status PUT aliases, Playwright forwards the real form request using PUT and the server handles the request. The report scenario observes the real per-account submission budget before submitting further invalid inputs.

## Game publisher evidence

The existing `specs/game-data.spec.mjs` from publisher commit
`b169fa9e8102e16fb6a3d09782b0749ca5a046e9` passed all four projects with zero
retries. It was run from an isolated local copy with `harness.mjs --game-data`
and the local Portal binary. Run the publisher spec with one worker because
its four projects publish the online snapshot for the same seeded world.
The publisher submits a real CreateCharacter receipt and an LCFA portfolio.
The publisher files were temporary and are outside the pushed branch.
