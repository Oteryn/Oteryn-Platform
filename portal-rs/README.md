# portal-rs

The Oteryn portal in Rust. The design and the task list are in `docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`.

This is the only platform. The earlier Laravel app, the Go gateway and the legacy import tool were removed on owner decision 2026-11-11: there is no cutover and no data migration, and the portal starts with an empty database. The last commit containing that tree is `2d45732c` (`legacy/`). Two reference checks keep what they needed from it: `crates/web/tests/ratelimit.rs` inlines the earlier portal's rate limits, and `crates/gateway/tests/fixtures/go_vectors.jsonl` keeps the captured Go vectors (`crates/gateway/tests/capture_go_vectors.py` regenerates them from that commit).

## Commands

```
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
OTERYN_DATABASE_URL=postgres://... cargo run -p oteryn-portal -- migrate
OTERYN_DATABASE_URL=postgres://... cargo run -p oteryn-portal -- serve
OTERYN_DATABASE_URL=postgres://... cargo run -p oteryn-portal -- bootstrap-intent issue --actor <staff-uuidv7> --input request.json  # operator issuer, audited, see the producer contract
```

## Native login protocol 3 and recovery (contract §19, testing only)

Gateway login contract §19 (N4P-4) runs only behind these default-off flags. Protocol 3 needs `OTERYN_ENV` `testing` or `preprod`; recovery handles also need `OTERYN_ENV=testing` (owner decision U21). Production stays refused.

| Flag | Where | Default | Effect |
|---|---|---|---|
| `GATEWAY_NATIVE_PROTOCOL3_ENABLED` | gateway | `false` | `protocol_version: 3` on `POST /v1/login`, and `POST /v1/recover` and `POST /v1/recover/revoke` relayed to the issuer (§19.4, §19.7; recovery also needs the issuer's `PORTAL_NATIVE_RECOVERY_ENABLED`). Needs `GATEWAY_NATIVE_LOGIN_ENABLED`, without which the recovery routes are `404`. Off: protocol 3 is `NATIVE_LOGIN_UNSUPPORTED_VERSION`. |
| `PORTAL_NATIVE_PROTOCOL3_ENABLED` | issuer | `false` | Protocol 3 on `native-admissions`: per-channel candidates (`game_channels.channel_key`), selection, the channel refusal and Decision M1. Every committed fresh attempt advances the account's play generation, whether this is on or off. |
| `PORTAL_NATIVE_RECOVERY_ENABLED` | issuer | `false` | Issues a recovery handle in protocol 3 login responses; otherwise `"recovery": null`. Also serves §19.4 `native-recoveries` (testing only). With binding on, every request and retry needs a `device_proof` from the handle's live device family. §19.7 `native-recoveries/revoke` answers whenever protocol 3 is on. |
| `PORTAL_NATIVE_RECOVERY_HANDLE_LIFETIME_SECONDS` | issuer | `43200` | Handle lifetime H, 3600..=43200. Any other value refuses protocol 3 issuance. |
| `PORTAL_NATIVE_RECOVERY_KEY_CUSTODY_FILE` | issuer | unset | §19.5 encrypted recovery key custody, distinct from the admission custody; trusted in the `existing_actor_recovery` scope. Unset or unreadable leaves recovery unavailable. |
| `PORTAL_NATIVE_RECOVERY_KEY_WRAPPING_KEY_FILE` | issuer | unset | 32-byte owner-only wrapping key of the recovery custody, distinct from the admission one. |
| `PORTAL_NATIVE_RECOVERY_REQUIRE_DEVICE_BINDING` | issuer | `true` | §19.15 binding. `false` is accepted only in `testing`; with binding on, a ticket without a live keyed device family gets `"recovery": null`. Binding on also needs `OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED=true`; otherwise protocol 3 issuance and recovery refuse. |
| `GAME_AUTH_NATIVE_RECOVERY_LOCATOR_IDENTITIES` | issuer | unset | §19.6.3 `POST /internal/v1/game-auth/native-recovery-locators` on the internal mTLS surface: a JSON object `{"<SAN>":["<world_id>/<channel_id>",...]}`, its SANs distinct from the runtime-status and evidence identities. Served only with `PORTAL_NATIVE_PROTOCOL3_ENABLED` and `PORTAL_NATIVE_RECOVERY_ENABLED` in `testing` or `preprod`. |

## Rules for every task

- One task changes one module crate (`crates/<module>`), plus its own migration file, templates under `templates/<module>/` and one line where its router is merged in `crates/web`.
- Migrations are additive: `migrations/<UTC timestamp>_<module>_<what>.sql`. Never edit a merged migration.
- No `unsafe`, `unwrap`, `expect` or `panic` outside tests. Errors are typed with `thiserror`.
- Every mutation is authorized server-side. Browser mutations need CSRF. Money uses a transaction with a row lock and an idempotency key.
- Game-facing `/internal/v1` routes keep the existing contracts byte for byte; tests read fixtures from `docs/contracts/fixtures/`.
- Features that touch the game or money are off by default behind a flag.
- Never commit secrets. Configuration comes from `OTERYN_*` environment variables.

## Game journal flags (T72)

All default to off; `bin/oteryn-portal` mounts the ingest only on a qualified internal mTLS listener and keeps refusing it in production.

| flag | effect when `true` |
|---|---|
| `PORTAL_PLAYERDATA_G10_INGEST_ENABLED`, `PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED` | G-10 / G-11 ingest admits records through the PostgreSQL ownership fence (`playerdata::authority::DbAuthority`) and the database clock (30 s / 5 s uncertainty bound); a baseline watermark promotes G-11 `game_login` records to the login history. Families also need `PORTAL_PLAYERDATA_JOURNAL_FAMILIES`. |
| `PLAYERDATA_G10_ITEMS_ENABLED`, `PLAYERDATA_G10_HISTORY_ENABLED` | With `PORTAL_PLAYERDATA_G10_SOURCE_AUTHORITY` set, item reads prove the current owner and cut, and still answer 503 until a G-10 baseline is verified and the catalogue mapping is accepted. |
| `OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED` with `OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED` | With `PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY` set, login history reads the promoted cut. |

Ownership comes from the character projection and a receipt chain: the committed `CreateCharacter` receipt is the initial cut (an accepted `pd_ownership_baselines` row only where no create receipt exists), and each committed transfer must name the previous owner as its seller. A pending transfer, missing creation evidence or a broken chain answers 503; a wrong owner or a stale cut answers 409. At the transfer cut the character's earlier login sessions are hidden from both accounts (G-11): the seller sees none of them, and the buyer sees only sessions after its cut. A sold character, or one whose ownership cannot be proven, is hidden from the login history; the rest of the page stays readable.
