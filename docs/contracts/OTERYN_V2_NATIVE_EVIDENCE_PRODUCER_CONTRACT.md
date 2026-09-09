# Oteryn V2 Platform native evidence producer contract

Status: **Platform counterpart accepted for implementation under Issue #1379; production deployment remains separately gated.**

Authority:
- Platform ADR 0028 owns canonical cross-boundary `AccountId` as immutable UUIDv7.
- Protected Game PR #470 (`ae22132fd9ddb6c83f5bf386fdc267cb670d16aa`) supplies the accepted consumer wire/ordering handoff for `FND-NATIVE-SOURCE-EVIDENCE-V1` and `FND-RECOVERY-SOURCE-TRANSPORT-V2`.
- This contract accepts that handoff on the Platform producer side without transferring Game, deployment, PKI or private-key authority.

## Producer operations

The private producer supports exactly:

| Version | Operation | Binding |
|---:|---|---|
| 1 | `ReadAccountSecurityV1` | canonical `account_id` plus independently configured fresh purpose/scope |
| 1 | `ReadFreshSigningTrustV1` | fixed issuer `urn:oteryn:platform:game-admission`, profile `oteryn-pre-admission-v1`, independently configured key purpose and bounded key id |
| 2 | `ReadRecoveryAccountSecurityV2` | canonical `account_id`, purpose `platform_security`, scope `existing_actor_recovery` |
| 2 | `ReadRecoverySigningTrustV2` | issuer `urn:oteryn:platform:game-recovery`, profile `oteryn-reauth-recovery-v1`, key purpose `existing_actor_recovery`, bounded key id |

Success and closed failure JSON shapes remain exact-compatible with the protected Game codec. Source revision, decision identity, source time, uncertainty and minimum valid generation are canonical decimal strings. Ed25519 public keys are exactly 32 bytes represented as 43-character unpadded base64url.

## Account identity and security truth

`identities.id` remains a local persistence surrogate. `identities.account_id` is a separately persisted lower-case UUIDv7, unique and immutable after creation/backfill. The expansion migration intentionally keeps the column nullable during a rolling deployment so a pre-AccountId writer cannot be broken by an immediate `NOT NULL` contract. Current writers always issue the UUIDv7. A later deployment phase, after every pre-AccountId writer is drained, must repeat the null-only backfill and only then enforce `NOT NULL`; the phases must not be collapsed into one rolling release. The backfill assigns only previously-null rows, so an already-persisted canonical AccountId is never reminted. AccountId is never derived from Canary identifiers or integer Identity ids.

`native_security_generation` is the positive Platform-native game security generation. It starts at `1` and is advanced transactionally in the same locked `RevokeIdentityGameAuthorizations` mutation that already serializes password, MFA, email, recovery and termination-sensitive revocations. The legacy `game_auth_generation` remains compatibility state; the producer reads `native_security_generation` directly and performs no `+1` translation.

Fresh V1 and recovery V2 account observations share one source-revision namespace per AccountId and the same security-generation floor. Every successful new observation advances that shared source ordering even if the security generation did not change.

## Observation durability and anti-rollback

A successful observation is persisted as an immutable exact JSON decision before it is returned. Its `source_revision` is positive and monotonic in the contract-defined namespace; `decision_identity` equals that revision. A revision is never re-aged or rewritten with changed facts/time.

The relational observation history is paired with retained high-water witnesses stored under `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`. That directory is required to be an independently retained durable volume, outside the relational database restore unit. A successful first observation installs revision `1`; database history above a missing witness fails closed. Database history below a retained observation witness is detected as producer rollback and the operation returns unavailable. Database history ahead of an existing retained witness may reconcile that witness upward because the relational decision is already the newer committed truth; witnesses are never automatically lowered or deleted.

Account security additionally retains the positive native security-generation high-water per AccountId. On first source activation it may bootstrap exactly to the current positive generation only when no prior source history exists. Once active, `RevokeIdentityGameAuthorizations` advances that independent generation witness before persisting the next generation; if retained witness configuration later disappears while relational source history proves activation, revocation itself fails closed rather than degrading to database-only state. A later database restore below the witness therefore makes the source unavailable rather than resurrecting an older authorization floor. Trust state uses the same safety rule: every profile/key mutation advances an independently retained profile-state witness before the relational mutation, and reads require exact equality between retained and relational state revisions. If a cross-store mutation becomes ambiguous after the witness advances but before the relational commit is proven, the namespace remains fail-closed; neither an application retry nor a restart may lower the witness. Explicit owner recovery/reconciliation is required before that namespace can serve authority again.

The application admits at most two in-flight producer operations and no application-side pending queue; this is a stricter server-side subset of the accepted `NSRC-INFLIGHT <= 2` and `NSRC-QUEUE <= 8` envelope. Authenticated peer throughput is additionally bounded by a service-specific per-peer limiter (`GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE`, strict range 1..600, default 120); exhaustion returns an empty, non-cacheable `429` transport failure. Request bodies are capped at 1,024 bytes, response bodies at 8,192 bytes, root JSON at 16 scalar members, headers at the accepted 8,192/32/2,048-byte shape, and nested/duplicate/unknown fields fail closed. Transfer/content encodings not safely accounted by this Laravel boundary are rejected rather than silently widened. After a request has decoded successfully, relational `QueryException` failures are normalized to the exact bounded `result: unavailable` wire shape instead of leaking framework exception JSON; non-database programming failures remain outside that conversion.

## Signing trust

Platform stores public trust only. It does not own or expose private signing material through this producer.

Each fixed `(issuer, profile, key_purpose)` trust set has a positive durable `issuer_revision` mirrored by the independent trust-state high-water witness. Each key id has immutable version rows with positive `key_revision`, canonical public key, explicit trusted state and revocation time. Key rotation/revocation locks the profile; profile revision and retained state witness advance together before the relational mutation becomes observable. A revoked key id cannot be silently re-trusted, and profile revocation cannot be bypassed by selecting another key id. Source observation revision is a separate profile/set-wide namespace shared across all key ids, as required by Game.

No Passport/OAuth key, Gateway bearer credential, Game Login Ticket key or Canary state is promoted into this trust authority.

## Private authenticated boundary

The route is private under `/internal/v1/game-auth/native-evidence`. Laravel requires upstream-authenticated TLS metadata `SSL_PROTOCOL=TLSv1.3`, `SSL_CLIENT_VERIFY=SUCCESS` and exact `SSL_CLIENT_S_DN` equality with the independently configured Game client identity. Missing configuration or verification fails closed. These server variables are trusted only when supplied by the separately qualified TLS terminator/FastCGI boundary; public headers are not accepted as substitutes.

Repository delivery does **not** claim a live mTLS listener, certificates, PKI roots, client private key, endpoint provisioning or connectivity. Real authenticated Platform↔Game qualification and deployment descriptor proof remain mandatory before WP5 S3.

## Configuration and exclusions

The producer is fail-closed until the deployment supplies a one-way activation decision and its required bindings. `GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED=true` is required before the first authoritative observation and must not be reverted after activation; revocation treats an activated producer without its witness as unavailable even if relational history has been restored backward.

The deployment supplies:
- `GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED=true` before cutover;
- `GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY`;
- `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` on an independently retained durable volume;
- fresh account purpose/scope and fresh signing key purpose matching the authorized Game descriptor;
- an explicit clock-uncertainty decimal in the strict range `0..5` (malformed, empty or boolean-like values fail closed);
- an optional native-evidence throughput limit in the strict range `1..600` requests/minute per authenticated peer (default `120`).

No production deployment, secret/certificate rotation, live credential mutation, Game repository mutation, character authority, Server Seam authority or legacy compatibility promotion is authorized by this contract.
