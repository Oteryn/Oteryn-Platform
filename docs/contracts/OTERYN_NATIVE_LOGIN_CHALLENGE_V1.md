# Oteryn native login challenge v1 (Platform reference copy)

Source: `Oteryn/Oteryn-Game` at commit
`42496a6650d60f7387fe44d6ab89a84394c70f21`, path
`docs/contracts/OTERYN_NATIVE_LOGIN_CHALLENGE_V1.md` (87 lines). The Game copy
is authoritative for the client side; this copy pins what Platform implements.
The only edit below is that the relative link to the Character boundary is
written as a path, because that file lives in Oteryn-Game.

## Owner decision for item 1 (2026-11-25)

Approved in chat ("zgoda") for "Exactly what Platform must implement", item 1:

- per identity: 5 failed sign-ins within 10 minutes trigger the challenge;
- per source: 20 attempts per minute trigger the challenge;
- source: the client address from `CF-Connecting-IP` only when the direct peer
  is a trusted Cloudflare/proxy address, reusing the repository's existing
  trusted-proxy/client-IP mechanism where one exists, otherwise the peer
  address;
- counter retention: 24 hours.

Thresholds are configuration with these defaults; counters are atomic across
replicas (PostgreSQL) and must not enable account enumeration.

## Platform implementation (portal-rs)

- Flag: `PORTAL_NATIVE_LOGIN_CHALLENGE` (JSON, absent means off; malformed
  JSON or an enabled policy without keys refuses the token route with 503).
  Members and defaults: `enabled` false, `identity_failures` 5,
  `identity_window_seconds` 600, `source_attempts` 20,
  `source_window_seconds` 60, `retention_seconds` 86400,
  `challenge_ttl_seconds` 120, `max_answer_attempts` 5, `policy_version` 1.
- Signing keys: `PORTAL_NATIVE_LOGIN_CHALLENGE_KEYS`, `kid=<base64url
  32-byte Ed25519 seed>` entries, comma-separated, provisioned outside the
  repository; the first signs, the others still verify (rotation).
- Code: `portal-rs/crates/gameauth/src/oauth/challenge.rs` (policy, counters,
  binding, proofs), `oauth/oauth_http.rs` (`/oauth/token` 403 envelope,
  `/oauth/challenge` page), `templates/oauth/challenge.html`; failed password
  sign-ins are recorded from `crates/identity/src/login_http.rs`.
- Schema: `portal-rs/migrations/20261125230000_native_login_challenge.sql`.
- Source address: the existing `TRUSTED_PROXIES` resolver
  (`oteryn_internal_api::proxy`), which walks `X-Forwarded-For` from trusted
  peers. `CF-Connecting-IP` is not read; whether to add it is an open owner
  question.
- Challenge content: a six-digit code shown on the page and spelled out in
  words as the text alternative; the browser must be signed in as the bound
  account. No third-party resource or script is loaded.
- Tests: `portal-rs/crates/gameauth/src/oauth/challenge_tests.rs`.

## Contract text (verbatim from the source commit)

Game-client implementation contract, ACCOUNT-SERVICES, 2026-11-08.
Platform implementation and deployment remain required. This is Oteryn's own
challenge; no CipSoft, Cloudflare or other external challenge service is contacted.
Identity/security policy stays in Platform under
the Character boundary (Oteryn-Game `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`).

## Client flow

The existing authorization-code + PKCE S256 flow is unchanged for ordinary login.
Before issuing an access token, `POST /oauth/token` may return HTTP 403 with exactly:

```json
{"error":"oteryn_challenge_required","challenge_id":"01890a5d-ac96-774b-bcce-b302099a8057","expires_in":120}
```

`challenge_id` is canonical lowercase UUIDv7; `expires_in` is an integer 1–300
seconds. Unknown/duplicate fields, wrong types and invalid bounds fail closed.
Bodies retain the existing 16 KiB OAuth bound. No access token, native ticket or
Game session exists yet. The code must remain usable for this specific retry.

The client opens the configured Platform origin's `/oauth/challenge` page in the
system browser with `client_id`, `challenge_id`, a **new** ephemeral IPv4 loopback
`redirect_uri` and independent 256-bit random `state`. It never opens a URL
supplied by the challenge response. The page should say “Oteryn sign-in check”
and explain completing the check and returning to the game. OAuth code, PKCE
verifier and access tokens are never placed in this URL.

After completion, Platform redirects to that exact loopback URI with
`code=<opaque challenge proof>&state=<exact state>`. `code` here is the existing
callback envelope's field name, not an OAuth authorization code. Proof is visible
ASCII using only letters, digits, `-`, `_` and `.` (unpadded base64url segments),
nonempty, at most 2048 bytes. This alphabet needs no query escaping and keeps
proof plus state within the existing 4096-byte callback query budget.
Existing callback validation rejects wrong
state, duplicate parameters, bad paths and malformed callbacks. Timeout is the
lesser of configured callback timeout and remaining challenge lifetime.

The client resubmits the **same** OAuth form (`grant_type`, `client_id`, original
OAuth `redirect_uri`, `code_verifier`, OAuth `code`) with two extra form fields:
`challenge_id` and `challenge_token`. A second challenge/refusal ends the attempt;
there is no automatic challenge loop. Browser failure, cancellation, wrong state
or expiry fails closed. Proof is redacted in Debug and exists only in memory.
Only a successful token response continues to character selection/ticket/admission.
The legacy `exchange_code` API fails closed on challenge responses.

## Exactly what Platform must implement

1. Count failed sign-ins and suspicious rates at the identity owner, atomically
   across replicas. An initial proposed policy is N=5 failures per identity in
   10 minutes or 20 attempts per source in one minute. Platform's owner must
   approve thresholds, trusted proxy/source handling and retention. Avoid account
   enumeration. A client counter or Game command cannot satisfy this requirement.
2. Durably bind the issued challenge to the pending authorization code, client,
   PKCE S256 challenge, original redirect URI, authenticated identity/context,
   expiry and policy version. Return 403 before code consumption; do not extend
   authorization-code expiry. Reject a retry after either expiry.
3. Serve `/oauth/challenge` with Oteryn-owned challenge content, accessibility
   alternative, rate limits and no third-party resources. Validate its client,
   challenge and loopback return URI against the pending flow; never allow an
   arbitrary redirect. Bind the return state/URI to this browser attempt.
4. Issue a short-lived, audience-bound signed proof after successful completion.
   Provision and rotate signing keys outside the repository; include a one-use
   nonce. Bind proof to the full pending flow above. Never accept a public test
   token, unsigned proof, response URL, client assertion or elapsed time as success.
5. On retry verify signature, audience, binding, nonce, expiry and current policy;
   consume proof and OAuth code and issue token in one transaction. Concurrent
   or replayed retries cannot issue another credential. Refuse invalid proofs
   with a generic OAuth error; no bypass after challenge expiry. Keep existing
   native-ticket scope/consumption and Gateway protocol unchanged.
6. Add Platform durable tests: N−1/N failures and suspicious rate; cross-replica
   counters; invalid/mismatched/expired/signature-invalid proof; concurrent
   consume, replay and rollback; no credential before completion; safe redirect
   validation; no secret logging. Deploy this extension before advertising it.

## Evidence in Game

`crates/platform-client/src/native_login/challenge.rs` owns bounded decoding and
proof-bearing exchange. `apps/client/src/native_entry/challenge.rs` owns the
browser/loopback/retry flow, called by production `authorize`.
Focused tests cover decoding, expiry, cancellation, redaction, repeated challenge,
browser failure and wrong callback state. The local Platform mock binds the exact
OAuth form and consumes its deterministic proof once; it is compiled only for
tests and is not a cryptographic verifier or a deployable identity service.
The admission test traverses real HTTP, loopback callback and TLS Game admission.
There are no Game SQL changes and no applicable Game PostgreSQL cases.
