# ADR 0045 — Native Gateway recovery grants and multiple channels per World

- Status: Proposed — 2026-10-12, design only; needs owner and Game architect acceptance
- Governing Issue: #1419 (N4-P native Game Gateway)
- Contract: `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` §19 (candidate amendment N4P-4)
- Consumes read-only: Game ADR-0003; Game `FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md`; Game `FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md`; Game `FND-RECOVERY-SOURCE-TRANSPORT-V2`; Platform ADR 0009, ADR 0020, ADR 0031, ADR 0044
- Does not authorize: code, migration, route, configuration, key, certificate, deployment, a production route record, or any Game repository change

## Context

The native login branch (protocol 2) issues one fresh-entry grant (`oteryn-pre-admission-v1`) for one Character on one channel. Two gaps remain.

1. **Recovery.** When a client loses its game connection and its game-domain reconnect proof (FND-04B §9), Game accepts only a Platform `oteryn-reauth-recovery-v1` grant (FND-04B §17). Game already serves `ClientResume` with that grant on the Channel owner (FND-04B §20, #822). Platform already produces the matching evidence (`ReadRecoverySigningTrustV2`, `ReadRecoveryAccountSecurityV2`). However, Platform has no way to issue the grant. Ticket issuance consumes the OAuth token family, so today the only way back is a new browser sign-in, a new ticket and a fresh admission. That fresh admission is the wrong credential for an actor that still exists.
2. **Channels.** Route records, runtime status and selection are already keyed by `(WorldId, ChannelId)`. However, the preproduction store publishes exactly one route record (ADR 0044). The login response also does not say how the channel was chosen or which channels were eligible. A refused channel preference returns the same code as "nothing is ready".

ADR 0009 rejected multichannel gameplay synchronization for the first release, but it required routing readiness for `channel_id`. ADR-0003 keeps route selection in the Platform Gateway and keeps the canonical `GameSessionId` in the game domain. ADR 0020 keeps one public Gateway hostname with `/v1/*` paths.

## Decision

### 1. Protocol 3, additive

A new native protocol version 3 carries both changes. Protocol 2 stays byte-for-byte as it is. The new public routes are `POST /v1/recover` and `POST /v1/recover/revoke` on the existing Gateway hostname. Each maps to one private issuer operation. The Gateway stays stateless and never assembles claims. Protocol 3 is default off and limited to `testing`/`preproduction`.

### 2. Recovery credential: a Platform recovery handle

A committed protocol 3 fresh admission also returns a **recovery handle**. The handle is:

- opaque, 256-bit and stored only as a hash;
- held in client memory only;
- bound to `AccountId`, `CharacterId`, `WorldId`, `native_security_generation`, a new per-account `native_play_generation` and an absolute expiry (proposed 12 h).

The handle can do exactly one thing: obtain an `oteryn-reauth-recovery-v1` grant for its bound Character and World through `/v1/recover`. It is never a ticket, a fresh grant, an OAuth token, or a credential a game node sees.

**Fencing.** Every committed fresh admission, under protocol 2 or 3, advances `native_play_generation`, which ends every older handle. A security-generation change, Identity disable, explicit revoke and absolute expiry also end a handle. These Platform fences are issuance-only. Game's `connection_generation`, `ControlLossEpoch`, RecoveryGrantNonce and healthy-controller rules stay the authority.

**Retirement before a new attempt.** At most one recovery attempt per account may be unretired at a time, until `exp + 5 s + clock_uncertainty`. Retrying the same `attempt_ref` returns the byte-identical grant. This meets the recovery profile §11 rule that a second independently usable grant needs deterministic retirement of the first.

**No rotation (R2).** The handle is not rotated on use. Rotating would need the successor in a response that can be lost. Returning that successor on retry would need either stored plaintext or a new derivation key. Theft exposure is bounded by:

- memory-only storage, the absolute expiry and the single live handle per account;
- the 30 s grant life;
- Game's one-time `jti`;
- Game's rule that a healthy controller is never preempted.

**Replace on fresh retry (R3).** A replay of the fresh attempt mints a new handle and revokes the previous one of that attempt, so no plaintext is ever stored.

### 3. Separate key purpose

Recovery grants use their own Ed25519 key pair in the recovery trust scope:

- issuer `urn:oteryn:platform:game-recovery`;
- profile `oteryn-reauth-recovery-v1`;
- key purpose `existing_actor_recovery`, fixed by Game;
- published through the existing trust registry rules.

The fresh scope keeps its configured key purpose (deployed as `fresh_admission`). A `kid` never appears in both scopes. A failing recovery scope stops only recovery. No reconnect key purpose exists: the fast reconnect proof is game-domain material that Platform never handles.

### 4. Recovery routing is evidence, not authority

The recovery grant carries no channel (recovery profile §7). Platform returns the endpoint of a routing target:

- a Game-reported recovery locator entry when one exists (dependency G1);
- otherwise the channel of the account's last committed attempt.

The target must pass the same readiness checks as fresh selection. Platform never substitutes another channel. Until G1 exists, no Game channel transition may be enabled together with recovery, because a moved actor would be unrecoverable through Platform.

### 5. Multiple channels per World

- **Distinct endpoints (M1).** One route record per `(WorldId, ChannelId)`, as in contract §7.3. Two current records never share `(host, port)`.
- **Response.** The protocol 3 login response adds `channel_selection` (`requested` or `automatic`) and `channels`, the eligible candidates as `{channel_id, key}` for display only.
- **Refusal.** A requested channel that is not eligible, while another is, returns `NATIVE_LOGIN_CHANNEL_UNAVAILABLE` with the eligible list. The ticket is unused, so the client retries with a new `attempt_ref`.
- **Selection rule.** Unchanged: requested if eligible, otherwise the lowest canonical eligible `channel_id`.
- **Preproduction store.** ADR 0044's one-record fence widens to every Channel of the one configured World. This replaces only ADR 0044's "exactly one route record" clause. Everything else in ADR 0044 stays: one World, the code allowlist and the audit.

## Alternatives considered

- **Recovery only after a full browser sign-in and new ticket.** Rejected as the only path, because it does not meet the goal. It stays the fallback when the handle is gone.
- **Recovery only through the remembered-device credential.** Rejected as the primary path. Remembering is opt-in, disabled by default, and its lost-response rule falls back to a browser sign-in. It may become a second entry (U23).
- **A rotating handle.** Rejected for protocol 3. See R2. Rotation needs stored plaintext or a separate derivation secret, and loses recovery on a lost response.
- **A fresh-entry grant for recovery.** Rejected. The Game profiles separate the two purposes, and the fresh grant is refused for an existing actor.
- **Signing the channel into the recovery grant.** Rejected. The recovery profile forbids channel authority, and Game resolves placement.
- **One `/v1/login` route with a purpose member.** Rejected. Separate routes keep the exact member sets, the rate limits and the issuer operations apart, and they cannot be confused by one field.
- **Shared endpoints across channels.** Deferred (U25). That would need node dispatch by grant, which is a Game change.

## Consequences

- A disconnected player recovers without the browser while the handle lives. Platform adds one hashed credential table, one per-account generation and one more signing key in custody.
- Platform must provision and rotate a second key, and publish recovery-scope trust.
- The client must keep the handle in memory only, and revoke it on logout.
- Multichannel Worlds become routable in preproduction with explicit channel feedback. Capacity-aware selection stays open (U6).

## Game dependencies (not changed by this ADR)

- **G1 (Game-node contract change):** a recovery locator feed reporting each Character placement change to Platform over a dedicated mTLS identity. It is needed before any channel transition is combined with recovery.
- **G2 (Game profile clarification):** the recovery profile §1 phrase "Platform freshly authenticated AccountId" covers a Platform recovery handle created by a fresh sign-in in the same play session and checked against the current security generation at issuance. Without G2, the recovery path needs a browser sign-in.
- **Client:** protocol 3 support in the Game client (`crates/platform-client`). This is not a node change.

## Rollout and rollback

See contract §19.11. Rollback switches protocol 3 off. Protocol 2 is unaffected, live handles stop working, and outstanding recovery grants expire within 30 s.

## Open questions for acceptance

U19–U25 in contract §19.12. The decisive ones are U21/G2 (handle as reauthentication), U20 (handle lifetime) and U22/G1 (locator).
