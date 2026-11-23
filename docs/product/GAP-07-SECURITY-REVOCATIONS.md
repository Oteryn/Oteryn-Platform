# GAP-07 · Game session revocation qualification

The portal records revocations when credentials change, access is recovered, account closure advances the security generation, an account restriction takes effect, or authorized staff request Game sign-out. A record confirms that the Platform transaction committed. Game termination and consumer progress currently show as unavailable.

## Platform operator checks

1. Open **Admin → Players**. The Game session revocations panel shows the export flag and the Game consumer status. Flag state describes configuration. Qualify transport separately on the private listener.
2. Open a player by account ID from the existing search. The player detail shows the latest ten committed revocations in stream order with their cause, UTC publication time and native security generation. The player can see their own recent causes and times under **Account → Web sessions**. Staff reasons remain in the staff audit.
3. In an isolated environment with the required authority, enable `PORTAL_SECURITY_REVOCATION_EXPORT_ENABLED=true` and configure `PORTAL_SECURITY_REVOCATION_EXPORT_IDENTITY` with a dedicated DNS SAN on the private mTLS listener. The flag defaults to off. It gates export and the staff sign-out action. Ordinary account security actions continue to record their revocations while export is off.
4. Use an MFA-confirmed staff account with `account.security.kick` to request Game sign-out for a UUIDv7 account. Verify the latest Platform record and the matching `players.kick` audit row. Use the dedicated mTLS identity to check that cursor replay returns the same records and malformed cursors are refused.

The [proposed stream contract](../contracts/OTERYN_V2_ACCOUNT_SECURITY_REVOCATION_STREAM_CONTRACT_PROPOSAL.md) governs the private endpoint, cursor format and capture semantics.

## Game work required for qualification

- Obtain consumer implementation and production authority. Register the numeric reconnect freshness bound and qualify the dedicated mTLS identity, durable cursor storage and replay handling.
- Apply revocations idempotently in stream order. Keep termination scoped to the account on the record. Terminate admissions with older security generations and honor the cause-3 rule when a restriction leaves generation unchanged.
- Refuse the terminated session's current and candidate reconnect proofs. During a stream outage, preserve current play and refuse fast reconnect when the freshness bound cannot be proven. Check that fresh eligible admissions still work.
- Exercise password change, email change, recovery, closure, account restriction and staff kick against a real admitted Game session. Include another account as a control, replay after a lost response and an outage. Record the consumer's actual outcome for each case. If Platform needs an applied status, agree on a receipt and its validation contract before exposing it.
