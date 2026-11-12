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

## Rules for every task

- One task changes one module crate (`crates/<module>`), plus its own migration file, templates under `templates/<module>/` and one line where its router is merged in `crates/web`.
- Migrations are additive: `migrations/<UTC timestamp>_<module>_<what>.sql`. Never edit a merged migration.
- No `unsafe`, `unwrap`, `expect` or `panic` outside tests. Errors are typed with `thiserror`.
- Every mutation is authorized server-side. Browser mutations need CSRF. Money uses a transaction with a row lock and an idempotency key.
- Game-facing `/internal/v1` routes keep the existing contracts byte for byte; tests read fixtures from `docs/contracts/fixtures/`.
- Features that touch the game or money are off by default behind a flag.
- Never commit secrets. Configuration comes from `OTERYN_*` environment variables.
