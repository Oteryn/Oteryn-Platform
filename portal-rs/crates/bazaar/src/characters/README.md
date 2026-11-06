# Character auctions (T59)

The production composition is now the [PLAT3 runtime](../game/PLAT3.md), selected
with the single `OTERYN_BAZAAR_ENABLED` flag. The description below records the
lower-level T59 projection boundary; its former standalone activation flag was removed.


The public DTO is a separate, bounded publication projection. It never serializes the raw G-7
signed envelope, ownership linkage, instance/lock/receipt IDs, bank balances or private proxy bids.
Supported empty sections remain visible; unsupported `None` sections remain absent. Item-key
search and anchor-based snapshot navigation work without JavaScript. English and Polish Fluent
catalogs and existing T39 components supply the UI. The contract exposes no transferable bank gold;
there is deliberately no gold tab. Unknown keys remain opaque keys, without invented semantics.

`CharacterMarket` composes T57's public transactional auction and reservation APIs. Bids recheck
current T58 evidence under T57's listing lock before touching funds. Listing submission requires
current authenticated account state, email verification, MFA, sudo and an owner-projection choice.
It records publication consent and an immutable local listing intent in one transaction. Retries
return the same draft; changed input conflicts. Requests are capped at 20/minute and 24 unfinished
per account, and concurrent duplicate Character listings are refused. These local intents are an
outbox seam, not canonical wire commands. No dispatcher is registered. They remain durable while
the game command flag is off. Publication requires the command gate, verified escrow, current trust,
consent and a validated immutable public snapshot. Revalidation failure hides a listing and denies
bids; it does not release custody or funds or invent a receipt outcome.

`browser_routes` supplies real T02 sessions and synchronizer CSRF/origin checks through
`SessionSecurity`. The lower-level `routes` accepts a security port for actor-to-result tests.
Form mutations refuse unknown fields; success redirects with 303. Public and owner pages are
no-store. Without `OTERYN_BAZAAR_ENABLED=true`, startup mounts the existing
localized 503 HTML refusal pages. With it enabled, the PLAT3 runtime supplies real
trust, custody, funding, transfer and delivery evidence. Configuration alone
never supplies missing Game facts.

## Dependency and contract boundaries

T57 and T39 are in the contract base. No T58 code or dependency branches were available at task
admission. `CharacterEvidence` is the narrow T58 seam: its implementation must use only durable,
transaction-locked verified evidence, never page-render game calls. `choices` must lock current
owner-projection and restore fences; `approved` must verify the exact listing, seller, Character,
lock, snapshot signature/hash/trust and current CHAR_CMD custody receipt. It must enforce the
30-day resale restriction and ownership cut using character_revision, projection_revision,
projection_epoch and recovery_generation, never an invented ownership counter. It returns only
separately approved public facts under current privacy policy. Missing, unknown or stale evidence
returns None. The game adapter and Wallet reservation adapter must be qualified before composing
production routes. Transfer/delivery/settlement, Game wire codecs and trust provisioning are supplied
by the PLAT3 runtime; the T59 projection library itself never dispatches commands.

Fail-closed decisions / remaining dependency work:

- G-7 capability advertisement and signing trust provisioning remain unspecified rollout gates.
- T57 has no atomic buy-now API; buy-now input is refused, rather than emulated with multiple
  transactions. Enable only after a qualified atomic API exists.
- Snapshot v1 has no item category or artwork catalog. Keys are searchable; no category, market
  valuation, automatic rarity label or sprite interpretation is invented. Search/filter/statistics
  beyond these snapshot details remain T61/T63.
- Rejected, ambiguous and reconciling Game states must be projected by T58 before richer owner
  status is available. A staged request is explicitly not a confirmed listing.

## Verification

The six PostgreSQL tests in `tests.rs` use PORTAL_TEST_DATABASE_URL, print a skip notice if unset,
and create/drop isolated schemas. They cover staged outbox requests and replay, publication and
funded bids, refusals/bad input, trust revocation, SSR privacy, real session CSRF/origin enforcement,
and listing form-to-owner-status flow. Unit tests cover scalar/projection bounds and both catalogs.
Runtime SQL uses bound parameters, with no new SQL query macros or offline metadata requirement.

The database-enabled workspace acceptance run is blocked by the unchanged
`admission::tests::db_gateway_client_calls_real_admission_listener`: Rustls cannot select a default
CryptoProvider when both ring and aws-lc-rs are enabled. The identical failure reproduces on the
original contract base 3b05567d53bb02080035f0e5ce518b22ef8d8119. T59 does not change that code or
provider configuration. Formatting, Clippy and all six new PostgreSQL tests pass.
