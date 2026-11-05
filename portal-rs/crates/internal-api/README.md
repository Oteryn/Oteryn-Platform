# Private contract boundary

`guard::protect(router, identities, policy)` installs a route-local Axum guard.
Give each contract purpose its own group (Store, Premium, native evidence,
account-character projection, runtime status, and so on). Do not group identities
merely by a shared URL prefix. Configure a separate policy for each body bound,
canonical shape and feature flag. `RoutePolicy::default()` is disabled.

Build `IdentityMap` from configured `Binding` values. An absent group returns 503;
duplicate identities are configuration errors, including duplicates within one
group. Treat a configuration error as unavailable, never retain a permissive map.
SANs are exact printable ASCII of 1–128 bytes; SPKI SHA-256 uses raw 32-byte digests.
Identity matching uses fixed-size constant-time comparisons and scans every binding.

The TLS terminator/connection adapter must verify the certificate chain, TLS 1.3
and HTTP/1.1 before inserting `VerifiedClient::from_verified_tls13_http11` into
request extensions. Incoming HTTP headers cannot establish this extension. This
crate does not implement TLS termination or trust roots; the constructor is a trust
boundary for the transport adapter, not a certificate verifier.

The guard checks availability, then identity, transport and bounded body collection,
then JSON. Only exact `application/json` is accepted. Transfer encoding and content
compression are refused. Peer refusal is an empty 401; unavailable is an empty 503,
with no application-added headers, challenge or diagnostics. Contract-specific
response headers, rate limits and operation errors belong to the owning route.

Canonical routes require a `canonical::Shape` defining ordered members recursively.
`decode_canonical` and `encode_canonical` enforce compact UTF-8 object bytes, no
escapes, unique members, schema order, depth and size. `JsonRule::Object` instead
permits member order and whitespace, as required by Premium snapshot requests.
Decoded `canonical::Value` is placed in request extensions and original body bytes
are preserved. Handlers must still validate values, source authority, scopes,
allowed operations, replay protection and business constraints.

The codec preserves insertion order, unescaped slashes and Unicode, and PHP's
escaping of U+2028/U+2029. Wire numeric tokens are signed-64-compatible integers;
uint64 quantities outside this range use contract decimal strings. Floats and
exponents are refused. Domain constraints may impose narrower numeric bounds.
Depth follows PHP `json_decode`: depth 4 permits three container levels.

`fixtures::load` is public for other crates' integration tests. It recursively
loads every JSON file, including schemas, manifests and invalid domain examples.
Pretty review artifacts compact in file member order; compact artifacts round-trip
exactly. Tests additionally verify all Store manifest byte lengths and SHA-256
vectors. A successful codec round-trip does not establish domain validity.

Run focused tests with `cargo test --locked -p oteryn-internal-api -- --nocapture`.
No database is used by this mechanic, so it adds no database integration tests or
SQL query macros. Real TLS interoperability and domain handlers remain separate
integration tasks; no game route is mounted or activated by this library.
