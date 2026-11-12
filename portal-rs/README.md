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
| `GATEWAY_NATIVE_PROTOCOL3_ENABLED` | gateway | `false` | `protocol_version: 3` on `POST /v1/login` (needs `GATEWAY_NATIVE_LOGIN_ENABLED`). Off: protocol 3 is `NATIVE_LOGIN_UNSUPPORTED_VERSION`. |
| `PORTAL_NATIVE_PROTOCOL3_ENABLED` | issuer | `false` | Protocol 3 on `native-admissions`: per-channel candidates (`game_channels.channel_key`), selection, the channel refusal and Decision M1. While on, every committed fresh attempt advances the account's play generation. |
| `PORTAL_NATIVE_RECOVERY_ENABLED` | issuer | `false` | Issues a recovery handle in protocol 3 login responses; otherwise `"recovery": null`. |
| `PORTAL_NATIVE_RECOVERY_HANDLE_LIFETIME_SECONDS` | issuer | `43200` | Handle lifetime H, 3600..=43200. Any other value refuses protocol 3 issuance. |
| `PORTAL_NATIVE_RECOVERY_REQUIRE_DEVICE_BINDING` | issuer | `true` | §19.15 binding. `false` is accepted only in `testing`; with binding on, a ticket without a live keyed device family gets `"recovery": null`. |

## Rules for every task

- One task changes one module crate (`crates/<module>`), plus its own migration file, templates under `templates/<module>/` and one line where its router is merged in `crates/web`.
- Migrations are additive: `migrations/<UTC timestamp>_<module>_<what>.sql`. Never edit a merged migration.
- No `unsafe`, `unwrap`, `expect` or `panic` outside tests. Errors are typed with `thiserror`.
- Every mutation is authorized server-side. Browser mutations need CSRF. Money uses a transaction with a row lock and an idempotency key.
- Game-facing `/internal/v1` routes keep the existing contracts byte for byte; tests read fixtures from `docs/contracts/fixtures/`.
- Features that touch the game or money are off by default behind a flag.
- Never commit secrets. Configuration comes from `OTERYN_*` environment variables.
