# Oteryn V2 Platform native evidence producer contract

Status: **Platform hardening candidate under Issue #1388; real composed non-production interoperability and production deployment remain separately gated.**

Authority:
- Platform ADR 0028 owns canonical cross-boundary `AccountId` as immutable UUIDv7.
- Protected Game `main@489e3e390a1bce1ce3439c66521ab75f8a826cd8` contains the current accepted consumer wire/ordering implementation for `FND-NATIVE-SOURCE-EVIDENCE-V1` and `FND-RECOVERY-SOURCE-TRANSPORT-V2`; the original accepted handoff was introduced through protected Game PR #470.
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

## Observation durability, retained-store provenance and anti-rollback

A successful observation is persisted as an immutable exact JSON decision before it is returned. Its `source_revision` is positive and monotonic in the contract-defined namespace; `decision_identity` equals that revision. A revision is never re-aged or rewritten with changed facts/time.

The relational observation history is paired with retained high-water witnesses stored under `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`. That directory is required to be an independently retained durable volume, outside the relational database restore unit. The qualified PHP/filesystem profile must expose working `fsync`; witness updates are accepted only after the temporary file has been written, flushed and `fsync`ed, atomically renamed, and the containing directory has also been `fsync`ed. Absence or failure of either file or directory synchronization fails closed.

The retained store has a random 256-bit `witness-store.id`. The same identifier is bound to the Platform database through the singleton `native_game_evidence_witness_stores` record. An existing database binding with a missing or mismatched retained marker fails closed, so an empty replacement writable directory cannot silently become authority. A retained marker with a missing database binding may re-establish that binding after a database restore because the retained store is the independent anti-rollback authority. An upgrade from the pre-provenance producer may adopt a configured directory only when it already contains a syntactically valid namespace `.floor` witness; database history with neither retained marker nor retained floor is not silently blessed.

A successful first observation installs revision `1`; database history above a missing witness fails closed. Database history below a retained observation witness is detected as producer rollback and the operation returns unavailable. Database history ahead of an existing retained observation witness may reconcile that witness upward because the relational decision is already the newer committed truth; observation witnesses are never automatically lowered or deleted.

Account security additionally retains the positive native security-generation high-water per AccountId. On first source activation it may bootstrap exactly to the current positive generation only when no prior source history exists. Once active, `RevokeIdentityGameAuthorizations` advances that independent generation witness before persisting the next generation; if retained witness configuration later disappears while relational source history proves activation, revocation itself fails closed rather than degrading to database-only state. A witness-ahead/database-behind generation caused by an outer security transaction rollback or database restore remains unavailable during normal producer reads. Recovery is an explicit owner operation: `game-auth:native-evidence:reconcile --account-id=...` may only move the Platform-native generation forward to the retained floor and refuses any state that would lower retained authority.

Trust state uses the same fail-closed rule. Every profile/key mutation advances an independently retained profile-state witness before the relational mutation becomes observable, and reads require exact equality between retained and relational state revisions. When a trust mutation becomes ambiguous after witness advance, normal reads and mutations remain unavailable. The explicit trust reconciliation path may only move the relational issuer revision forward to the retained floor while conservatively leaving the affected profile version revoked. Re-enabling trust then requires an explicit successor profile version and a fresh key id; neither reconciliation nor retry clears an existing revocation.

## Capacity and throughput semantics

The application admits at most two in-flight producer operations and no application-side pending queue; this is a stricter server-side subset of the accepted `NSRC-INFLIGHT <= 2` and `NSRC-QUEUE <= 8` envelope.

`GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE` is a service-specific per-peer **best-effort application throughput guard**, with strict configuration range `1..600` and default `120`. The Laravel limiter's check/callback/hit behavior is not represented as a hard atomic concurrent-admission reservation and must not be used as proof of the two-in-flight bound. Exhaustion returns an empty, non-cacheable `429` transport failure. The hard application concurrency claim comes from the independent two-slot `NativeEvidenceCapacity` admission boundary.

Request bodies are capped at 1,024 bytes, response bodies at 8,192 bytes, root JSON at 16 scalar members, headers at the accepted 8,192/32/2,048-byte shape, and nested/duplicate/unknown fields fail closed. Transfer/content encodings not safely accounted by this Laravel boundary are rejected rather than silently widened. After a request has decoded successfully, relational `QueryException` failures are normalized to the exact bounded `result: unavailable` wire shape instead of leaking framework exception JSON; non-database programming failures remain outside that conversion.

## Signing trust lifecycle

Platform stores public trust only. It does not own or expose private signing material through this producer.

Each fixed `(issuer, profile, key_purpose)` trust set has a positive durable `issuer_revision` mirrored by the independent trust-state high-water witness. Internally, immutable `profile_version` rows preserve terminal profile-revocation history while keeping the external issuer/profile/key-purpose contract fixed. Each key id has immutable version rows with positive `key_revision`, canonical public key, explicit trusted state and revocation time.

Normal key rotation or key revocation operates only inside the current non-revoked profile version. A revoked key id cannot be silently re-trusted. Terminal profile revocation cannot be cleared in place. After an ambiguous witness-ahead state is conservatively reconciled revoked, or after an intentional terminal profile revocation, a successor trust set is created only through `publishNextProfileVersion`: it increments both the internal profile version and the retained issuer revision, preserves the old revoked row, requires a key id never used in an earlier version of that fixed scope, and installs only public key material. The producer reads only the highest profile version for the fixed external scope.

Source observation revision is a separate profile/set-wide namespace shared across all key ids, as required by Game. No Passport/OAuth key, Gateway bearer credential, Game Login Ticket key or Canary state is promoted into this trust authority.

## Private authenticated boundary

The route is private under `/internal/v1/game-auth/native-evidence`. Laravel requires upstream-authenticated TLS metadata `SSL_PROTOCOL=TLSv1.3`, `SSL_CLIENT_VERIFY=SUCCESS` and exact `SSL_CLIENT_S_DN` equality with the independently configured Game client identity. Missing configuration or verification fails closed. These server variables are trusted only when supplied by the separately qualified TLS terminator/FastCGI boundary; public headers are not accepted as substitutes.

Repository feature tests that inject application server variables prove middleware interpretation only. They are **not** composed mTLS evidence. Real non-production qualification must use an actual TLS 1.3 mutual-authentication path, prove that untrusted public input cannot synthesize the trusted server variables, exercise all four producer operations with an exact pinned Game consumer, and bind the evidence to exact Platform and Game revisions. The procedure is recorded in `docs/agents/evidence/OTERYN-20260912-platform-native-evidence-hardening/REAL_INTEROP_QUALIFICATION.md`.

Repository delivery does **not** claim a live production mTLS listener, certificates, PKI roots, client private key, endpoint provisioning or production connectivity. Real authenticated Platform↔Game qualification remains mandatory before Game WP5 S3 can claim composed readiness.

## Configuration and exclusions

The producer is fail-closed until the deployment supplies a one-way activation decision and its required bindings. `GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED=true` is required before the first authoritative observation and must not be reverted after activation; revocation treats an activated producer without its witness as unavailable even if relational history has been restored backward.

The deployment supplies:
- `GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED=true` before cutover;
- `GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY`;
- `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` on an independently retained durable volume whose qualified PHP/filesystem profile supports file and directory `fsync`;
- fresh account purpose/scope and fresh signing key purpose matching the authorized Game descriptor;
- an explicit clock-uncertainty decimal in the strict range `0..5` (malformed, empty or boolean-like values fail closed);
- an optional native-evidence best-effort throughput limit in the strict range `1..600` requests/minute per authenticated peer (default `120`).

No production deployment, secret/certificate rotation, live credential mutation, Game repository mutation, character authority, Server Seam authority or legacy compatibility promotion is authorized by this contract.
