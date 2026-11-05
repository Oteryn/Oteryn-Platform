# Native bootstrap API

T17 owns `POST /api/v1/game-auth/tickets` and
`GET /api/v1/game-auth/native-characters`. Governing wire contracts are
`docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` sections 4 and 5.3
and design section 5.2. Golden shapes are checked against the Game client at
`857f15f376907c592bb28d854fccff18a15cdf87`.

The base already includes T18. No T16 branch existed at task admission.
`configured_router` therefore mounts disabled routes with `DenyOAuth` and no
projection. It cannot grant access through configuration alone. The coordinator
must supply the T16 adapter through `Api` and `Tickets::new`, plus the T18
`Projection`, when composing enabled routes. A T16 branch appeared during final
verification at `901074f627bbac39d98b704d79d02e8bbe190b90`; its public
`verify_access` owns a separate pool transaction and does not expose atomic
access/refresh consumption or native security generations. Calling it here would
not satisfy the transaction contract. The explicit authority seam remains denied
until those facts can be verified on the caller's connection. Both settings are off by default;
bootstrap remains restricted to testing or preproduction.

`OAuthAuthority::verify` runs on the caller's transaction connection. It must
lock and verify the token, first-party client and authoritative account, check
scope, expiry, revocation and security generation, and return explicit active
account state. Consumption must revoke the access token and its refresh tokens
on that same connection. Ticket persistence and consumption commit together.
Character reads use `consume=false`, bind the projection to that account and
revalidate account and generation after the projection read. Unknown state denies.

Tickets contain 256 random bits, encoded without padding. Only SHA-256 digests
persist, with the native audience, account, both generations, expiry and future
redemption fields. A subsequent admission adapter must lock the ticket, verify
its audience, expiry, unused state and current account generations, then mark it
used on the admission transaction connection. T17 does not authorize admission.

PostgreSQL tests create isolated schemas and read `PORTAL_TEST_DATABASE_URL`.
They print a skip notice if it is absent. No compile-time SQL query macros or
new offline query metadata are required.
