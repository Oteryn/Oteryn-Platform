# Character Authority commands enablement (Game contract §14 step 4)

Owning contracts:

- Game `oteryn-game-character-authority-commands-v1`
  (`Oteryn/Oteryn-Game` `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md`).
- Platform `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`.

This runbook is the step-4 enablement packet. It lists the switches that turn on web Character creation and
the Game consumer, and what must be true before each is flipped. It flips nothing itself. Every flag below
defaults to off. The owner flips them on one stack at a time: first `testing` with shared fixtures, then
`preproduction`. Platform honours the command settings only when `APP_ENV` is `testing` or `preproduction`
(`CharacterCommandSettings::current()`), so production stays off whatever its environment says.

## Gates that must be closed first

| Gate | State | Evidence |
|---|---|---|
| U-CC7: Platform stores `recovery_generation` and handles the restore notice (PLATFORM-RESTORE-RECONCILE-P1) | open until Platform PR #1629 (code) and the contract PR #1631 merge and the owner accepts the contract | §13 U-CC7 |
| U-CC9: intents committed behind the consumer cursor per 30 s rescan interval | closed: Platform issuance commits none (below) | `CharacterCommandIssuanceOrderPgsqlTest` |

### U-CC9 proof (Platform side)

- `CharacterCommandClient::issue()` takes the issuer lock before it draws the row id, which is the
  `source_revision`. The lock is `pg_advisory_xact_lock` on PostgreSQL and a locking count on MariaDB.
  The lock is transaction-scoped, so it is held until the outermost transaction commits. That includes
  `CreateCharacter`'s outer transaction.
- Fresh intents therefore commit in `source_revision` order. The Game consumer's cursor is always the key of
  an intent it has already listed, so it is a committed key. Every later commit lies above it.
- The number of fresh intents committed behind the cursor per 30 s interval is therefore 0. The §4.1 TTL
  proof (up to 41 unseen keys) is never needed for fresh issuance.
- A sequence left behind its table would break this. That happens after a copy or restore that skipped
  `setval`, or when a row was written by hand. In that case the drawn id is not above every stored key, so
  `issue()` refuses the intent before it can be listed:
  - it throws `CharacterCommandUnavailable`, which the account page maps to "creation unavailable";
  - it logs `character_authority.command_source_revision_behind` at error level.

  To recover, re-seat the sequence:

  ```sql
  select setval(pg_get_serial_sequence('character_command_intents','id'), (select max(id) from character_command_intents));
  ```
- Intents reopened after a Game restore (§5.4, U-CC7) do land below the cursor. This happens once per
  restore notice, not once per interval, and §5.4 expects them to be decided by operation identity
  (in practice `CHAR_CMD_INTENT_EXPIRED`). They are outside the U-CC9 per-interval cap.

## Preconditions (deploy wiring, not done by this packet)

1. **Certificates.** Issue a dedicated client certificate for the Game character-command consumer. Its subject
   must not be used by any other internal purpose, such as evidence, runtime status, projection, bootstrap
   intents or the restore notice. Platform refuses an identity list that shares a subject
   (`CharacterCommandSettings`). Game refuses the evidence, runtime-status and projection files
   (`node/config.rs`).
2. **nginx.** In `deploy/synology/nginx/internal-mtls.conf`, the internal mTLS listener's location regex
   exposes only the existing routes. Add `character-commands/pending|character-commands/read|character-commands/receipt`.
   Add `game-restore/notice` when the restore flags are set.
3. **Compose.** `deploy/synology/compose.yml` passes an explicit environment list to the `platform` service. Add
   the Platform variables below to that list; a variable set only in `.env` does not reach the container.
4. **Smoke.** After deploy, the Game consumer's first pending-list exchange must answer `200`
   `{"contract_version":1,"pending":[]}`. A `503` means disabled or misconfigured; a `401` means the identity or
   the source authority does not match.

## Platform switches

| Variable | Value | Note |
|---|---|---|
| `CHARACTER_AUTHORITY_COMMANDS_ENABLED` | `true` | intent store and the three Game-initiated routes |
| `CHARACTER_AUTHORITY_COMMAND_IDENTITIES` | `'["CN=<GAME_CHARACTER_COMMANDS_IDENTITY>"]'` | JSON list; the subject from precondition 1 |
| `CHARACTER_AUTHORITY_COMMAND_SOURCE_AUTHORITY` | `oteryn:character-authority:primary` | must equal Game `[platform.character_commands].source_authority` |
| `CHARACTER_AUTHORITY_WEB_CREATE_ENABLED` | `true` | account page "Create character"; flip last, after the smoke check |
| `CHARACTER_AUTHORITY_COMMAND_INTENT_TTL_SECONDS` | unset (300) | 1..300 |
| `CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS` | unset (40) | 1..41; a value above 41 disables commands (see the backlog decision below) |
| `CHARACTER_AUTHORITY_COMMAND_REQUESTS_PER_MINUTE` | unset (600) | per consumer identity |

The restore-notice switches arrive with #1629: `CHARACTER_AUTHORITY_RESTORE_NOTICE_ENABLED`,
`CHARACTER_AUTHORITY_RESTORE_NOTICE_IDENTITIES`, `CHARACTER_AUTHORITY_RESTORE_NOTICE_SOURCE_AUTHORITY` and
`CHARACTER_AUTHORITY_RESTORE_NOTICE_REQUESTS_PER_MINUTE`. The Game repository has no restore-notice sender at
the time of writing. Leave these off until one exists, then set them with their own dedicated identity.

## Game switch

The Game consumer has no environment flag. It runs only when the stack's `node.toml` has the
`[platform.character_commands]` table (`apps/game-server/src/node/serve.rs`). Add it after
`[platform.account_characters]`, as the template `deploy/synology-game/node.toml.template` does for the
projection:

```toml
[platform.character_commands]
client_certificate_file = "<BASE>/node/secrets/character-commands.crt"
client_key_file = "<BASE>/node/secrets/character-commands.key"
source_authority = "oteryn:character-authority:primary"
```

Place the files in `BASE/node/secrets`, mode 0600, owner 1001, as for the other node identities. Restart the
node. The consumer then lists, reads and publishes receipts over the same Platform endpoint and trust roots as
`[platform]`.

## Order

1. Merge U-CC7 (#1629, #1631) and record the owner's acceptance.
2. Apply preconditions 1 to 3 and deploy Platform with `CHARACTER_AUTHORITY_COMMANDS_ENABLED`, the identities
   and the source authority. Leave `CHARACTER_AUTHORITY_WEB_CREATE_ENABLED` off.
3. Add `[platform.character_commands]` to the Game node and restart it. Run the smoke check (precondition 4).
4. Set `CHARACTER_AUTHORITY_WEB_CREATE_ENABLED=true` and redeploy Platform.
5. Repeat on `preproduction` after `testing` passes the contract §11 scenarios.

Rollback: set `CHARACTER_AUTHORITY_WEB_CREATE_ENABLED=false` first, which stops new intents. Then remove the Game
table. Leave `CHARACTER_AUTHORITY_COMMANDS_ENABLED` on until the pending list is empty, so every stored intent
gets its receipt. The migration rollback is refused while intents exist.

## Backlog decision

Decision: 40, enforced by Platform (values above 41 disable commands).

The forward path is a backlog that the consumer has not yet seen, with no rescan involved. At 6 s per unseen
key (read plus receipt), the contract §4.1 arithmetic (300 s TTL less 30 s rescan and 24 s listing) keeps 41
keys inside the intent TTL. A larger backlog would let the last intent of a burst wait past 300 s and be
decided `CHAR_CMD_INTENT_EXPIRED`. `max_pending_intents` therefore defaults to 40, and
`CharacterCommandSettings` makes the whole command surface unavailable for a configured value above 41
(`CharacterCommandWire::MAX_PENDING_BACKLOG`, test `CharacterCommandBacklogBoundTest`). The wire ceiling
`CHARCMD-PENDING-BACKLOG` stays 256.
