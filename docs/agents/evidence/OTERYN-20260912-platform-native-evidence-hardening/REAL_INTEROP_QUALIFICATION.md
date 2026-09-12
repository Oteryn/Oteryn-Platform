# Platform native evidence — real interoperability qualification

Status: `PROCEDURE_READY / COMPOSED_RUN_NOT_YET_CLAIMED`

Governing Platform Issue: #1388.

Repository-qualified Platform runtime candidate: `Oteryn/Oteryn-Platform@a3a44b781fc8131bf4b85f054f860c570071dcbe`.

This is a non-production evidence procedure. It does not authorize production deployment, PKI/root changes, certificate or private-key custody changes, secrets in the repository, Game repository writes, live user/account/session mutations, ruleset changes or Merge Queue bypass.

## Pinned consumer baseline inspected during hardening

- Platform admission base: `Oteryn/Oteryn-Platform@2271fea9db1e202cacb206dab289efa4cefcec76`.
- Repository-qualified Platform runtime candidate: `Oteryn/Oteryn-Platform@a3a44b781fc8131bf4b85f054f860c570071dcbe`.
- Game protected main inspected and re-read at repository-readiness closeout: `Oteryn/Oteryn-Game@489e3e390a1bce1ce3439c66521ab75f8a826cd8`.
- Game wire fixture: `apps/game-server/tests/admission_evidence_wire.rs` at that exact Game revision.
- Game native source codec/client: `apps/game-server/src/admission_evidence.rs` at that exact Game revision.
- Producer route: `POST /internal/v1/game-auth/native-evidence`.

The exact Platform candidate SHA must be frozen again at composed execution time. Game protected main must also be re-read immediately before the composed run; if either revision moved, the exact producer/consumer revisions actually used by the run replace the repository-readiness pins above. A Platform-local PASS never becomes a cross-repository PASS by prose.

## Required environment

Use an isolated non-production Platform database and a dedicated retained witness volume. The volume must be outside the database restore unit and the exact PHP/filesystem/container profile must successfully provide file and directory `fsync`. Use synthetic canonical AccountIds and synthetic public signing keys only.

Use a real TLS 1.3 mutual-authentication termination path between the pinned Game-side client fixture/harness and the Platform origin. The TLS terminator must validate a dedicated non-production Game client certificate against its configured trust anchor and must overwrite/supply the trusted FastCGI/application metadata itself. Do not persist certificate private keys, bearer credentials or trust-root secrets in repository evidence.

## Mandatory provenance checks

Before treating `SSL_CLIENT_VERIFY`, `SSL_PROTOCOL` or `SSL_CLIENT_S_DN` as evidence, prove all of the following against the exact non-production terminator/origin configuration:

1. unauthenticated public requests cannot supply headers or parameters that become trusted `SSL_*` server variables;
2. a request without a client certificate is rejected before the producer runs;
3. a certificate outside the accepted client trust chain is rejected;
4. a valid client certificate over a protocol below TLS 1.3 is rejected for this route/profile;
5. the accepted certificate subject reaches the application as exactly the independently configured `GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY`;
6. sanitized logs/evidence identify the terminator configuration revision and Platform candidate SHA without recording private key material or full secrets.

Application feature tests that directly inject server variables satisfy none of these composed provenance checks by themselves.

## Four-operation compatibility run

From the exact pinned Game consumer/harness, execute and decode all four operations against the real non-production route:

1. `ReadAccountSecurityV1`;
2. `ReadFreshSigningTrustV1`;
3. `ReadRecoveryAccountSecurityV2`;
4. `ReadRecoverySigningTrustV2`.

For each operation record only non-secret bounded evidence: exact Platform SHA, exact Game SHA, operation/version, HTTP status, response byte count, decoded result class, source revision and decision identity. Do not record private keys, client credentials, bearer material or live account identifiers.

The Game decoder must accept the real Platform response without fixture rewriting. Platform must preserve the current protected Game request/response binding, canonical decimal strings, 43-character unpadded Ed25519 public key encoding and closed failure shapes.

## Bounds and failure qualification

On the same exact candidate/environment prove:

- request body over 1,024 bytes is rejected;
- response remains within 8,192 bytes;
- duplicate, nested, unknown or malformed fields fail closed;
- wrong/missing mTLS provenance fails before authority is served;
- `NativeEvidenceCapacity` never admits more than two application operations concurrently and does not create an application-side pending queue;
- `GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE` is recorded only as the best-effort Laravel throughput guard and is not used as evidence for the hard concurrent-admission ceiling;
- relational query failure after successful decode returns the bounded `result: unavailable` shape.

## Durability, restore and recovery qualification

Use only disposable synthetic data.

1. Create account and trust observations and record their retained floor/state files plus the `witness-store.id` digest/identifier only as non-secret metadata.
2. Restart the Platform process/container without replacing the witness volume and prove the same store identity and monotonic floors remain authoritative.
3. Restore the Platform database to a snapshot that omits only the witness-store database binding while retaining the witness volume; prove the retained `witness-store.id` re-establishes the same database binding.
4. Replace the configured witness directory with a new empty writable directory while retaining database history/binding; prove producer reads fail closed and the replacement is not silently blessed.
5. Create the account-security ambiguous window: advance the retained generation through the shared revocation action inside a larger transaction, force the larger transaction to roll back, restart, and prove normal reads remain unavailable. Run the explicit account reconciliation command and prove it only moves the native generation forward to the retained floor; then prove observation resumes with that floor.
6. Create an analogous signing-trust witness-ahead/database-behind rollback. Prove normal reads remain unavailable; run `game-auth:native-evidence:reconcile --trust=fresh` (and independently the recovery scope when applicable); prove reconciliation marks the affected profile version revoked at the retained issuer revision. Restore trust only by creating a successor profile version with a fresh key id and prove the previous revoked profile remains immutable history.
7. Inject file-sync or directory-sync failure in a disposable qualified harness/profile and prove no successful authority response is claimed from that failed persistence step.

### Restore topology boundary

The anti-rollback design deliberately has two independent authority records: relational native-evidence provenance/history and the separately retained witness volume. Genuine first activation is distinguishable from ordinary database restore or retained-volume replacement while at least one of those independent records survives.

Simultaneous loss or rollback of **both** authority records removes the evidence needed to distinguish a historical producer from a genuinely unused installation. That topology is not a qualified restore path and must be classified `BLOCKED`; it must never be represented as successful provenance recovery, real interoperability proof or production proof. Operational backup/restore design must preserve at least one independent authority record across recovery.

## Evidence result classes

Use only these result classes for the composed run:

- `COMPOSED_PASS` — every mandatory item above passed on one explicitly pinned Platform candidate SHA, Game consumer SHA and non-production topology revision;
- `COMPOSED_FAIL` — an executed mandatory check failed;
- `BLOCKED` — the required real mTLS/retained-volume/non-production environment, exact consumer revision or qualified restore topology was unavailable;
- `NOT_RUN` — procedure exists but composed execution has not occurred.

Until an actual run records `COMPOSED_PASS`, the strongest repository-only terminal statement is `PLATFORM_NATIVE_EVIDENCE_READY_FOR_REAL_INTEROP`; it is not `REAL_INTEROP_PROVEN` and never `PRODUCTION_PROVEN`.
