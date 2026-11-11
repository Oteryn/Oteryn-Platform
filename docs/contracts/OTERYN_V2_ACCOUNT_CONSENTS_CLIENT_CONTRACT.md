# Account consents browser contract

Status: **PROPOSED**

Scope: optional account permissions, independent of terms and privacy acceptance.
No game protocol or game-server contract changes are made by this document.

## Client entry point

The game client opens the system browser at the portal's configured HTTPS origin
plus `/pl/account/consents` or `/en/account/consents`. It must not embed a password
form, forward access tokens in the URL, or introduce a consent API route. The
browser uses its own portal session; an anonymous visitor is redirected to the
localized login page. After login the user can reopen the consent page.

## Purposes and decisions

- `play_session`: permission to save play sessions for the sole purpose of
  improving the gaming experience or enforcing the Rules.
- `personalised_ads`: permission to use account data to show personalised ads on
  various external platforms. There are no ads in the game.

Both permissions default to **not granted**. Each form independently grants or
revokes one permission. Every successful submission appends a decision to the
existing `account_consents` table: account, kind, granted boolean, text version,
UTC timestamp, actor equal to the account, and source `web`. The current decision
is the latest timestamp, with the record ID breaking ties. No history means no
permission. Application writes are serialized on the account row; database
triggers refuse changes to or deletion of optional decisions.

Text is provided by the Polish and English Fluent catalogs in the identity
consent module. `OptionalConfig` supplies the corresponding version strings;
bump a purpose's version whenever either translation changes. A grant must carry
the current version. An old form can still revoke permission, recorded against
the current text version. A text version change does not silently grant any
permission or rewrite earlier decisions.

`OTERYN_ACCOUNT_CONSENTS_ENABLED=true` enables the browser surface. It is off by
default. This flag collects decisions only; it enables neither recording nor
advertising. Any later consumer must have its own default-off feature flag.

## Browser mutation contract

POST to the same localized page using `application/x-www-form-urlencoded`:
`kind`, `version`, `_csrf`, and optionally `granted=true`. An unchecked checkbox
omits `granted` and means revoke. The account and actor come exclusively from the
session. Unknown or duplicate fields and unsupported kinds/values are refused.
The existing portal session middleware verifies the synchronizer CSRF token and
origin checks before mutation. Success returns a 303 redirect to the page.

Refusals: 400/422 malformed input, 403 CSRF or inactive account, 409 outdated
grant text, 404 disabled feature or unsupported locale, 503 storage unavailable.
No failed submission appends a decision. The page displays the current permission
and decision date in UTC, or explicitly says that no decision has been recorded.

Account data export retains legal acceptance and includes the optional decision
history with the granted state, version, timestamp, actor and source. It uses the
existing owner-only export authorization and snapshot; no separate download
route is added.

## Future game-server exposure (out of scope)

A later, separately reviewed authenticated internal portal-to-game contract may
expose the current `play_session` decision, version and decision timestamp for a
bound account. That consumer must treat absent state or an unavailable portal as
not granted and propagate revocation before further recording. The exact route,
wire shape, caching and revocation delivery are deliberately undecided here.
No endpoint or game-server integration is implemented in this task. The client
must not infer consent from a browser redirect or locally cached toggle state.
