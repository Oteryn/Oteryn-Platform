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
| `/{lang}/account/privacy` (POST and PUT) | Yes | Yes | Anonymous | Yes | Each save advances the revision |
| `/account/badges` and `/{lang}/account/badges` | Account-age selection and clearing | Yes | Anonymous | Yes | Selection unchanged after replay |
| `/{lang}/account/notifications` | Yes | Yes | Anonymous | Yes | Same preferences after replay |
| `/{lang}/unsubscribe` | Signed confirmation form | Signed capability missing or forged | Possession of the signed link grants access | Malformed and overlength capabilities | No additional change on replay |
| `/{lang}/support/tickets` | Yes | Yes | Anonymous | Yes | Request-key replay |
| `/{lang}/support/tickets/{id}/reply` | Yes | Yes | Foreign player | Yes | Stale lock version |
| `/{lang}/support/tickets/{id}/status` (POST and PUT, close and reopen) | Yes | Yes | Foreign player | Yes | Stale lock version |
| `/{lang}/admin/support/tickets/{id}/reply` (public reply) | Yes | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/admin/support/tickets/{id}/status` (POST resolve and PUT reopen) | Yes | Yes | Foreign player and content editor | Yes | Stale lock version |
| `/{lang}/support/tickets/{id}/read` | Yes | Yes | Foreign player and content editor | Yes | Unread flag clears once |
| `/{lang}/support/reports` | Yes | Yes | Anonymous, and report stays private from another player | Yes | Request-key replay |
| `/{lang}/shop/coins` | Yes | Yes | Anonymous | Yes | Operation replay |
| `/{lang}/shop/transfer` | Yes | Yes | Content editor with a separate wallet | Yes | One debit and one credit |
| `/{lang}/shop/test-checkout/{intent}` (pay) | Yes | Yes | Foreign player | Yes | One event and one credit |
| `/{lang}/shop/test-checkout/{intent}` (reject) | Yes | Yes | Foreign player | Yes | One event and zero credits |
| `/{lang}/shop/test-checkout/{intent}` (cancel) | Yes | Yes | Foreign player | Yes | One event and zero credits |
| `/{lang}/account/characters/new` | Queued, and publisher confirms creation | Yes | Anonymous | Yes | Submission replay |

The documented unavailable views cover collection deck creation/deletion, battle start/turn/forfeit, season-pass purchase/claim, daily puzzle answers, and collection PvP and set purchases. The harness has no accepted collection ownership/progress adapter, season-pass authority adapters, daily puzzle catalogue, or the collection PvP gameplay and wallet adapters. The standard matrix proves that the character creation form persists one pending command. The publisher matrix proves the terminal receipt, durable portfolio, and visible creation result.

Additional refusal boundaries were verified in all four projects. Hunt sharing
returns 503 because the browser composition closes the share route pending its
accepted party privacy scope. The shop purchase form is absent because the
configured browser shop has no authoritative Game ownership adapter and
sets purchases off. A signed-in browser receives 503 on the attempted purchase,
and no store order is persisted. Stats watch POST and DELETE refuse browser
cookies with 401 and no subscription. These routes are bearer-only and the
configured policy denies all bearer principals until the T66 adapter is wired.

The publisher matrix also asserts a–d for both profile methods and both route
prefixes. It uses a different player and an account with the content editor
role as non-owners, requires localized validation for invalid inputs, and
reads the stored profile after each form submission. Profile saves advance
the privacy revision, so the replay criterion is inapplicable.

The journal report page returns 503 because its production `DenySource`
adapter supplies no accepted Game journal authority. The matrix checks the
absent weekly form and the valid-session note submission's 404 with no row.
The email preference route still exposes its account-local write. The matrix
checks CSRF and anonymous refusal, localized input errors, the persisted opt-in
and opt-out, and replay of the same opt-in. The unavailable report page prevents
a browser-form success for either journal row.

The standard harness produces an inactive Game catalogue with no accepted
world version. Its atlas returns 503 without annotation forms. The matrix
exercises the actual annotation endpoints with the browser session, including
marker creation, private reads, cross-account deletion refusal and replay,
and creation of a pending map report. The places participation API also saves
and exposes its account-local choice while the `UnresolvedSource` adapter
leaves the parent page and its form unavailable.

The fake Bazaar runtime mounts the item catalogue as GET-only and exposes
its own sell form. The legacy catalogue POST returns 405 and each of its eight
item actions returns 404 because the configured live Bazaar router replaces
those legacy routes. The tests check that no legacy listing is persisted.

Recruitment creation requires an accepted character profile snapshot and a
fresh `character_profile` watermark in addition to LCFA ownership. The Game
publisher fixture supplies LCFA but does not publish the profile family. The
matrix checks the valid-session 403 from creation and the 404 from reporting
or removing an absent post, with no persisted post.

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

To run the profile matrix, place the publisher commit’s `harness.mjs` and
`game-publishers.mjs` in an isolated local copy containing this matrix.
Run `harness.mjs --game-data` in that copy, then pass its report path to
the following command from `portal-rs/e2e`.

```sh
PORTAL_E2E_GAME_DATA_REPORT=/absolute/path/report.json \
  node node_modules/@playwright/test/cli.js test \
  --config mutation/publisher.config.mjs --workers=1 --retries=0
```
