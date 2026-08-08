# Native pre-admission handoff architecture review — 2026-08-08

## Result

`ARCHITECTURE_BOUNDARY_RESOLVED — IMPLEMENTATION / CONSUMER / CUTOVER NOT IMPLIED`

## Evidence reviewed

- accepted Platform ADR 0028 — canonical native `AccountId` boundary;
- `docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md`;
- accepted Platform ADR 0031;
- current focused `OTERYN_V2_INTEGRATION_ARCHITECTURE.md`;
- current `GAME_GATEWAY_IDENTITY_CONTRACT.md`;
- historical/transitional `OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`;
- accepted `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`;
- read-only accepted Oteryn-v2 ADR-0003 Platform Identity/Game Gateway/admission boundary.

## Finding

Platform and Oteryn-v2 already agreed on authority direction, but Platform lacked a focused target semantic contract for the object crossing from authenticated/routed Gateway orchestration into native game admission. The unresolved gap could permit future implementation to conflate a Platform authorization attempt with the game-owned canonical gameplay session, reuse legacy Canary IDs/semantics, or mishandle replay/ambiguous outcomes.

## Resolution

`docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md` freezes the Platform-side semantics without inventing unfinished Oteryn-v2 implementation details.

The contract establishes:

- Game Login Ticket, native pre-admission material and canonical game-domain `GameSessionId` are distinct lifecycle objects;
- Platform authorization is one bounded attempt, not proof of final gameplay admission;
- canonical `AccountId`, `CharacterId`, `WorldId`, `ChannelId` and route/revision context replace Canary numeric identities in the native boundary;
- issuance depends on authoritative ticket redeem, safe current character authorization evidence, World Registry policy and fresh applicable current-owner runtime evidence;
- audience/route/revision binding, short lifetime, one-success anti-replay semantics and ambiguous-outcome handling are mandatory;
- Oteryn-v2 revalidates authoritative ownership/lifecycle and owns final admission, character lease, fencing, canonical `GameSessionId`, reconnect and recovery;
- Channel switching requires fresh destination authorization;
- no password/OAuth/Canary fallback is permitted;
- exact transport/encoding/signing, TTL value, replay store, FND-04 state machine, lease/fencing algorithm and canonical `GameSessionId` wire form remain external/deferred.

## AccountId authority reconciliation

Orphan-takeover review found one material documentation risk that the earlier self-review had missed.

The delivered private redeem v1 described by `GAME_GATEWAY_IDENTITY_CONTRACT.md` is Canary-compatible and returns/binds `canary_account_id`. Accepted ADR 0028 and `OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md` are narrower and authoritative for the native target:

```text
native Game Login Ticket account binding = AccountId
native redeem/login context minimum       = AccountId + security_generation + redeemed_at
canary_account_id                          = legacy compatibility / ACL identifier only
```

Therefore:

- successful legacy Canary v1 redeem is not, by itself, proof that the native AccountId-bearing redeem/login context exists;
- the current `/internal/v1/game-auth/tickets/redeem` payload remains delivered Canary compatibility behavior unless/until a separately authorized versioned native context is implemented;
- native pre-admission issuance may proceed only from an authoritative native login context that yields canonical `AccountId` under ADR 0028 semantics;
- native `AccountId` must never be reconstructed from `canary_account_id` merely because a compatibility mapping exists;
- Issue #888 does not implement or activate that native redeem context.

This is not a new authority. It is the direct application of already accepted ADR 0028 / `OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md` to the pre-admission boundary.

## Reconciliation

- `OTERYN_V2_INTEGRATION_ARCHITECTURE.md` routes admission semantics to the focused contract, moves the item from P1 deferred backlog to a resolved focused boundary and preserves implementation/cutover nonclaims.
- `GAME_GATEWAY_IDENTITY_CONTRACT.md` explicitly stops native target authority at Identity/Gateway authorization/orchestration and routes post-redeem native handoff semantics to the focused contract while retaining Canary-compatible response evidence as compatibility state.
- ADR 0028 / `OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md` remain authoritative where the historical Gateway contract binds account authority to `canary_account_id`.
- historical Platform native Game Session v2/protocol artifacts remain reconciliation evidence only and receive no new target authority.

## Nonclaims

This review does not prove or authorize:

- a native AccountId-bearing ticket/redeem runtime implementation;
- native pre-admission producer runtime implementation;
- Oteryn-v2 consumer implementation;
- exact cross-repository envelope/schema/transport compatibility;
- game lease/fencing implementation;
- protocol-oteryn activation;
- staging E2E;
- production activation;
- any write to Oteryn-v2.

## Self-review after orphan takeover

```yaml
result: PASS
scope_paths:
  - docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
  - docs/agents/reports/OTERYN-20260808-native-pre-admission-handoff.md
material_findings:
  - finding: legacy Gateway redeem v1 uses canary_account_id while native ADR 0028 requires AccountId
    disposition: RESOLVED_BY_AUTHORITY_RECONCILIATION
    evidence: ADR 0028 and OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT are explicitly narrower/authoritative for native identity; current v1 remains compatibility and native implementation remains unproven
unresolved_material_findings: []
authority_conflicts: []
negative_paths_checked:
  - replay/duplicate use
  - ambiguous issuer outcome
  - ambiguous game-admission outcome
  - stale ownership projection
  - stale runtime/route owner generation
  - wrong route/channel/revision
  - account-security change after issuance
  - channel switching
  - reconnect/recovery
  - legacy password/OAuth/Canary fallback
  - legacy canary_account_id accidentally promoted to native AccountId
compatibility_checked: true
rollback_checked: true
external_repository_writes: false
runtime_browser_e2e: NOT_APPLICABLE
```

## Remaining external unknowns

- exact accepted Oteryn-v2 FND-04 admission/session state machine;
- exact native AccountId-bearing redeem/login-context endpoint/version and runtime implementation;
- exact pre-admission envelope schema/encoding/transport/signing primitive;
- exact short TTL value and security-revocation-after-issuance mechanism;
- exact atomic consume/replay mechanism;
- exact character lease/fencing algorithm;
- canonical `GameSessionId` wire representation;
- exact producer/consumer compatibility revision and native E2E evidence.
