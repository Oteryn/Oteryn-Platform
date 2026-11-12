# Native remembered-device client contract

> Superseded for request and response bodies by the ACCEPTED `OTERYN_V2_NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT.md`: rotation and revocation carry no `rotation_proof` and responses carry no `rotation_nonce`.

Status: **PROPOSED**. Default-off. This is the Rust client qualification proposal
for N008, including the M2 transport and L3 rotation gates. It changes the historical
protocol-1 keyless rotation behavior; clients must qualify against this proposal
before the flag can be enabled. It grants no production rollout authority.

`OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED` defaults to false. Rust additionally
requires `APP_ENV=testing|preproduction`, valid enabled `PORTAL_OAUTH_CONFIG`, an
explicit enabled first-party/public/native-ticket row in `oauth_native_clients`,
and live account/security authority. Production always refuses. No client is opted
in by the migration. Remembering a device requires explicit consent on that login.

Transport must reach the process through its direct TLS 1.3 listener, configured
with all of `OTERYN_NATIVE_TLS_BIND`, `OTERYN_NATIVE_TLS_CERT_FILE`, and
`OTERYN_NATIVE_TLS_KEY_FILE`. Partial configuration prevents startup. HTTP/1.1 is
supported. The ordinary HTTP listener refuses these operations even on loopback.
TLS terminated at a proxy followed by HTTP is insufficient; neither an HTTPS URL
nor `X-Forwarded-Proto`, `Forwarded`, or `X-Forwarded-For` grants transport trust.
The source address is always the accepted socket peer. Clients verify server
certificates and hostnames using platform trust, with no certificate bypass.

## Endpoints and exact members

All endpoints use POST and `Content-Type: application/json`. Base path:
`/api/v1/game-auth/device-sessions`. No query string is accepted. The Authorization
header appears exactly once, with one ASCII space and the case-sensitive scheme.
First sign-in uses the system browser and OAuth Authorization Code + PKCE S256,
with `game:ticket` scope. Passwords, MFA, and recovery stay in the browser.

| Path | Authorization | Request type | HTTP 200 response type |
| --- | --- | --- | --- |
| `/api/v1/game-auth/device-sessions` | `Bearer` plus OAuth access token | EnrollmentRequest | CredentialResponse |
| `/api/v1/game-auth/device-sessions/native-characters` | `OterynDevice` plus current credential | OperationRequest with rotation_proof | CharactersResponse |
| `/api/v1/game-auth/device-sessions/tickets` | `OterynDevice` plus current credential | OperationRequest with rotation_proof | TicketResponse |
| `/api/v1/game-auth/device-sessions/revoke` | `OterynDevice` plus any known family credential | OperationRequest without rotation_proof | RevokeResponse |

Every request has integer `protocol_version:1`. Enrollment has exactly four
members: that version, boolean `remember_device`, `device_public_key` (canonical
unpadded base64url of a 32-byte Ed25519 public key), and `enrollment_proof`
(canonical unpadded base64url of its 64-byte signature). Both true and false require
a key; the historical two-member remembered request is refused with 400. Existing
unbound families cannot rotate; obtain a fresh browser login and enroll a new key.
Families predating the nonce migration must also sign in again; no challenge is
backfilled into an existing client. An OAuth access token enrolls at most one family. A public key belongs to one family
and cannot be reused for a later enrollment, including after revocation.

The enrollment signature covers exactly the UTF-8 compact JCS object with members
in this lexical order: `access_token_sha256`, `client_id`, `persistence`,
`public_key`, `purpose`. Their values are lowercase hex SHA-256 of the exact OAuth
bearer (without its scheme), canonical lowercase client UUID, `remembered` when
consent is true or `session` otherwise, the encoded public key, and the literal
`oteryn-native-device-enroll-v1`. There is no trailing newline. No client/account
identity, scope, generation, or purpose authority can be supplied in the body.

This synthetic enrollment example uses bearer
`AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA` and client UUID
`019a5f4a-1000-7000-8000-000000000001`. Examples contain no live credentials.

<!-- example:EnrollmentRequest -->
```json
{"protocol_version":1,"remember_device":true,"device_public_key":"6kpsY-KcUgq-9VB7Ey7F-ZVHdq6-vnuSQh7qaRRG0iw","enrollment_proof":"lZWhZ_o6xAGIWG_-dVXNOMlvTE3V_PM_XNDlueDRDKiMdS1G-5VZwk1__pXjqKASJL8F_JBPo0lrxBZ0jZw5Dw"}
```

CredentialResponse has exactly `protocol_version` (integer 1), `device_credential`
(string: `otd1.` followed by 43 canonical base64url characters, 32 random bytes),
`family_id` (UUID), `absolute_expires_at` and `idle_expires_at` (integer UTC Unix
seconds), and `rotation_nonce` (43 canonical base64url characters, 32 random bytes).
The nonce is public but must not be reused after a successful rotation.

<!-- example:CredentialResponse -->
```json
{"protocol_version":1,"device_credential":"otd1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8","family_id":"019a5f4a-1000-7000-8000-000000000002","absolute_expires_at":1796997600,"idle_expires_at":1795010400,"rotation_nonce":"ICEiIyQlJicoKSorLC0uLzAxMjM0NTY3ODk6Ozw9Pj8"}
```

OperationRequest has `protocol_version`, `client_id` (canonical lowercase RFC4122
UUID, versions 1 through 8), and, for rotation, `rotation_proof`. The proof has
exactly `nonce`, `session_id`, `timestamp`, `device_public_key`, and `signature`.
`session_id` is the canonical UUID from `family_id`; `nonce` is the server value
from the most recent credential response; `timestamp` is signed integer UTC Unix
seconds; the key and signature have the same encodings/sizes as enrollment.

<!-- example:OperationRequest -->
```json
{"protocol_version":1,"client_id":"019a5f4a-1000-7000-8000-000000000001","rotation_proof":{"nonce":"ICEiIyQlJicoKSorLC0uLzAxMjM0NTY3ODk6Ozw9Pj8","session_id":"019a5f4a-1000-7000-8000-000000000002","timestamp":1794405600,"device_public_key":"6kpsY-KcUgq-9VB7Ey7F-ZVHdq6-vnuSQh7qaRRG0iw","signature":"wCFa-qSVXu2_OTha678f8AHjtd2-eJt-S6Vbf3Osiv8aVnItvXBPmZS3-lv6khs0002zAGppqbQj_vi9VhSRBw"}}
```

## Rotation and proof bytes

There is no separate refresh endpoint. **Every successful characters or ticket
operation rotates the credential and nonce**, in the same PostgreSQL transaction
as its native effect. The client must serialize all such requests across threads
and processes. It signs exactly this compact UTF-8 object in lexical member order
(`nonce`, `purpose`, `session_id`, `timestamp`), with no newline:

<!-- example:RotationMessage -->
```json
{"nonce":"ICEiIyQlJicoKSorLC0uLzAxMjM0NTY3ODk6Ozw9Pj8","purpose":"oteryn-native-device-rotate-v1","session_id":"019a5f4a-1000-7000-8000-000000000002","timestamp":1794405600}
```

The purpose literal is always `oteryn-native-device-rotate-v1`. Strict Ed25519
verification uses the enrolled key. Server database time and the submitted timestamp
must differ by at most 60 seconds in either direction (inclusive). The nonce must
match the current family challenge and the session UUID must match that family.
The challenge lasts until the next committed rotation or family expiry/revocation;
a fresh timestamp and signature are required even if the challenge was issued earlier.

Missing proof, malformed proof, stale/future proof outside the bound, a foreign
session UUID, an invalid signature, or a consumed nonce refuses the operation.
Missing/stale/replayed proofs leave the family and unrelated families intact. A
fresh matching nonce/session proof naming a different validly encoded public key
revokes that family durably, even if its signature is invalid; unrelated families
are untouched. Freshness is checked before key comparison. Simultaneous proof reuse
permits at most one success and cannot consume the successor. A consumed credential
presented with a fresh proof over the current nonce revokes the family (credential
reuse). A wrong-client or unknown credential never revokes a family.

CharactersResponse contains all CredentialResponse members plus `characters`, an
array of at most 64 objects, each with exactly `character_id` (UUID), `world_id`
(UUID), `name` (1..96 UTF-8 bytes), and `availability` (`AVAILABLE` or `UNAVAILABLE`).
Only the authenticated owner's projection is returned; missing or stale projection
state refuses with 503. The configured source authority and matching highest feed
epoch/account snapshot are required. Feed freshness is at most 30 seconds with
one second uncertainty; missing projection-authority configuration refuses reads.

<!-- example:CharactersResponse -->
```json
{"protocol_version":1,"device_credential":"otd1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8","family_id":"019a5f4a-1000-7000-8000-000000000002","absolute_expires_at":1796997600,"idle_expires_at":1795010400,"rotation_nonce":"ICEiIyQlJicoKSorLC0uLzAxMjM0NTY3ODk6Ozw9Pj8","characters":[{"character_id":"019a5f4a-1000-7000-8000-000000000003","world_id":"019a5f4a-1000-7000-8000-000000000004","name":"Player","availability":"AVAILABLE"}]}
```

TicketResponse contains all CredentialResponse members plus `ticket` (nonempty
string, at most 256 bytes), `expires_in` (integer seconds, 1..60), and `expires_at`
(integer UTC Unix seconds, strictly after issuance and no later than 60 seconds).
The ticket is single-use and bound to the device family by the real ticket issuer;
it is not a device credential. Never persist it or reuse it after admission.
Each response example illustrates its shape independently; they are not a sequence
of reusable credentials. Successful rotation always returns a new credential/nonce.

<!-- example:TicketResponse -->
```json
{"protocol_version":1,"device_credential":"otd1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8","family_id":"019a5f4a-1000-7000-8000-000000000002","absolute_expires_at":1796997600,"idle_expires_at":1795010400,"rotation_nonce":"ICEiIyQlJicoKSorLC0uLzAxMjM0NTY3ODk6Ozw9Pj8","ticket":"native-ticket-example-20261111","expires_in":30,"expires_at":1794405630}
```

Revoke requires exactly the two-member OperationRequest; an included proof, even
null, is invalid. A known predecessor is sufficient for idempotent revocation.

<!-- example:OperationRequest -->
```json
{"protocol_version":1,"client_id":"019a5f4a-1000-7000-8000-000000000001"}
```
<!-- example:RevokeResponse -->
```json
{"protocol_version":1,"revoked":true}
```

## Errors and limits

All errors contain exactly one string member, `error`. No input, key, credential,
account detail, or exception is echoed.

| Status | error | Meaning / client action |
| --- | --- | --- |
| 400 | invalid_request | Shape, encoding, duplicate/unknown member, query or content framing is invalid; fix the request. |
| 413 | invalid_request | Declared or actual body exceeds 1024 bytes. |
| 401 | device_authorization_unavailable | Credential, rotation proof, expiry, account, generation, or client refused; clear local authority and sign in again. |
| 403 | device_authorization_unavailable | Enrollment key signature refused; do not retry unchanged. |
| 409 | device_authorization_unavailable | OAuth enrollment token or device key already enrolled; obtain a fresh login and key. No plaintext recovery. |
| 429 | too_many_requests | Source limit; Retry-After: 60. Serialize requests and wait. |
| 503 | device_authorization_unavailable | Disabled, production, transport/configuration/authority/database unavailable. No fallback to HTTP or keyless refresh. |

401/403/409/503 use this identical body:

<!-- example:ErrorResponse -->
```json
{"error":"device_authorization_unavailable"}
```
<!-- example:ErrorResponse -->
```json
{"error":"too_many_requests"}
```
<!-- example:ErrorResponse -->
```json
{"error":"invalid_request"}
```

Every response carries `Cache-Control: no-store, private` and `Pragma: no-cache`.
The default durable source throttle is 30 requests per 60 seconds, configurable
only within 1..60; every endpoint and unsuccessful authenticated request shares
that source budget. IPv4 uses the peer IP; IPv6 uses its /64 prefix. Forwarding
headers do not change the bucket. The TLS listener permits at most 64 concurrent
connections, five seconds to handshake and 60 seconds per connection.

Bodies have at most 1024 bytes, a flat enrollment/revoke object or one nested proof
object. Reject unknown members, duplicates (including escaped duplicate names),
wrong types, null proof, compressed bodies except explicit identity encoding,
transfer encoding, multiple content-type/length/encoding headers, inconsistent
lengths, nondecimal or noncanonical content lengths, and query strings. JSON
whitespace and member order are accepted. Responses are bounded to 16384 bytes.
Authorization is at most 8192 bytes. There are at most three internal retries for
confirmed database serialization/deadlock aborts; ambiguous commits never retry.

## Client storage and revocation

Remembered consent yields at most 30 days absolute and seven days idle, configurable
downward. A rotation extends idle to min(now + idle interval, original absolute);
it never extends the absolute deadline. Session class (`remember_device:false`)
is memory-only, with both deadlines fixed to enrollment + 43200 seconds; rotation
extends neither. Equality with either expiry is expired.

Generate a fresh key for each enrollment. For remembered devices use an OS credential
vault for the credential and an OS vault/non-exportable key store for the private key,
scoped to the trusted origin and client ID. Do not persist OAuth refresh tokens,
OAuth access tokens, tickets, or private keys in settings, logs, URLs, cookies,
crash reports, or plain files. If secure storage is unavailable, offer only the
in-memory session class and explain that sign-in is needed after exit. The server
cannot attest storage quality; this is a client obligation.

Atomically replace the stored credential/nonce before using a rotation result.
On an uncertain response or failed rotation, discard local authority and return to
browser sign-in; never blindly retry a predecessor or reuse a proof. 429 before
execution does not consume authority, but do not infer non-consumption from network
failure. On native sign-out revoke the family and erase the vault/memory state even
if the server is unreachable. Revoke is idempotent with any known family credential,
including consumed predecessors; unknown credentials return 401. Ordinary browser
logout is not redefined as native sign-out.

Password/email/security changes, MFA confirmation/replacement/removal, account
suspension, changed security generations, explicit owner revocation, and credential
reuse fence families. Account reactivation never revives them. Issued tickets and
already admitted sessions retain their own generation/admission checks. Family
revocation does not invent a new game-session termination mechanism. Missing live
security witnesses fail closed. Browser owner revocation requires authenticated
ownership, CSRF, and recent authentication in its owning adapter.

Production enablement, client implementation/OS-vault qualification, recovery after
process restart, and retained-witness operational qualification remain separate
release work. This proposal does not authorize them.
