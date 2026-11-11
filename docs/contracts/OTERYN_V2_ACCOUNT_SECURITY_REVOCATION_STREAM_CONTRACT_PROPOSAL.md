# Account security revocation stream V1 (proposal)

- Status: **PROPOSED**, with owner decisions 1 and 2 accepted (2026-11-11, relayed by the programme coordinator; see *Owner decisions*). It authorizes no implementation, migration, route or flag yet.
- `IMPLEMENTATION_AUTHORITY: NONE`; `PRODUCTION_AUTHORITY: NONE`.
- Owner: Platform (producer) with the Game session owner (consumer).
- Origin: FND-04B-AMEND-1 owner decision 5 (2026-11-11), relayed by the programme coordinator. A Platform ban, kick or password change must make the Game session terminal, so the Game fast-reconnect proof (`oteryn-game-reconnect-v1`) dies with it. This must be shown by an end-to-end test.

## Gap this closes

Platform never calls Game (plan §5.3.2). Today, Platform-side security changes reach a live Game session in these ways:

| Platform change | Platform effect today | Reaches a live Game session? |
| --- | --- | --- |
| Password, email or verification change, account status change, MFA requirement change, confirmed MFA replace/remove | `accounts.game_auth_generation` and `native_security_generation` advance (trigger `fence_native_device_credentials`, migration `20261109080401`). Device sessions are revoked. New tickets and admissions are refused. | No |
| Recovery-key use, account termination | `native_security_generation` advances (identity crate) | No |
| Account suspension or ban (T34 `suspension`, typed `ban`) | `require_game_access` refuses new tickets and admissions. The generations do not change. | No. N009 (`OTERYN_V2_SANCTION_EXPORT_CONTRACT.md`) exports a player-visible record, and Game "does not enforce decisions from this feed". |
| Staff kick | No such action exists in portal-rs | No |

`OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md` covers moderator-requested game sanctions. Its transport is deferred, and it does not cover credential changes.

## Proposal

### Transport

`POST /internal/v1/identity/security-revocations/stream/read`. It is served on the existing private TLS 1.3 / HTTP/1.1 listener only, and the identity is the verified client certificate's single DNS SAN, as for N009. The proposed configuration is:

- `PORTAL_SECURITY_REVOCATION_EXPORT_ENABLED`, default off;
- `PORTAL_SECURITY_REVOCATION_EXPORT_IDENTITY`, a dedicated SAN that differs from every other internal identity and takes part in the shared overlap check.

All of the following are identical to N009:

- status bodies and status codes;
- size, depth and JSON rules;
- rate budget;
- `Cache-Control`;
- cursor format (`v1:<decimal>`) and cursor persistence rules;
- the 256 KiB byte budget.

Request: `{"schema":"oteryn.security_revocation_stream_request.v1","after":null|cursor,"limit":1..500}`.

Response member order: `schema` (`oteryn.security_revocation_stream_page.v1`), `items`, `next_cursor`.

Each item, in this exact member order:

| Member | Rule |
| --- | --- |
| revocation_id | UUIDv7, unique per record |
| account_id | UUIDv7 |
| native_security_generation | positive signed 64-bit JSON integer: the account's generation after the change; non-decreasing per account in stream order |
| cause | integer: 1 credential (password, email, verification, MFA), 2 status (closed, terminated, recovery), 3 sanction (account ban or suspension became effective), 4 staff_kick |
| effective_at_ms | positive Unix milliseconds of the Platform commit |

The stream carries no reason text, staff identity, IP, device or credential data.

### Publication

- A trigger appends the record in the same transaction as the change, under a single counter-row lock. This gives the same ordering guarantee as N009: a larger position cannot become visible before a smaller one commits. A rollback removes both the counter increment and the record.
- Sources:
  - every `accounts` update that advances `native_security_generation`;
  - every sanction revision that makes an account-level ban or suspension effective;
  - the staff kick action below.
- Expiry or lifting of a sanction publishes nothing. Revocation is one-way, and the player simply logs in again.
- Capture continues while export is disabled. History is not pruned.

### Staff kick (Platform admin action)

This is a new admin action, `kick`. Its only effect is to advance `native_security_generation` with cause 4. It requires:

- a dedicated moderator/admin RBAC permission;
- confirmed MFA;
- an audit row (actor, target, time, reason) in the same transaction.

It changes no account status, sanction or credential. It also revokes device sessions through the existing trigger.

### Consumer obligations (Game)

- Poll this stream, and N009 for display only.
- Apply records in position order. Idempotency key: `revocation_id`.
- On a record for account A with generation G, make terminal every Game session of A whose admission carried a native security generation lower than G. Then refuse its current and candidate `oteryn-game-reconnect-v1` proofs (FND-04B-AMEND-1, refusal `RECONNECT_PROOF_INVALID` / `RECONNECT_NOT_ELIGIBLE`).
- Cause 3 also applies when the generation did not change. In that case, all sessions of A become terminal.
- An empty page means the consumer is caught up. It does not prove that nothing was revoked.
- If the stream is unavailable, Game keeps current play and retries. It fails closed for fast reconnect: without proof of a read within its registered freshness bound, the reconnect is refused and the player re-authenticates through Platform (decision 3).

## Required end-to-end test (acceptance gate)

For each cause (1 password change, 3 account ban, 4 staff kick):

1. A player is admitted and holds a current Game reconnect proof.
2. Platform commits the change.
3. Game polls this stream and applies the record.
4. Game makes the session terminal, and a reconnect with the held proof is refused.
5. A fresh Platform login after the cause is resolved (new password, lifted ban, or after the kick) is admitted with the new generation.

Additional cases:

- a cursor replay after a lost page applies each record once;
- a rolled-back Platform transaction publishes nothing;
- a stream outage blocks no current play and revives no session.

## Owner decisions

1. **Accepted (owner, 2026-11-11).** This stream is the decision-5 delivery path for every session-ending cause, bans included. In-game penalties that do not end the session (for example mute or jail) may still use `OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md`.
2. **Accepted (owner, 2026-11-11).** The staff kick action exists, gated by a dedicated moderator/admin RBAC permission. Every use writes an audit row recording actor, target, time and reason, in the same transaction as the generation advance. The reason stays in the Platform audit and never appears on the stream.
3. **Game decision (2026-11-11): fail closed.** If Game cannot prove that it has read this stream within its freshness bound, it refuses fast reconnect, and the player re-authenticates through Platform. Game registers the numeric bound with its implementation.

Still open before implementation: Platform implementation authority for the producer and the kick action, and Game implementation authority for the consumer.
