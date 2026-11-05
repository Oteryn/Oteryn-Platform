# portal-rs

The Oteryn portal in Rust. The design and the task list are in `docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`.

## Commands

```
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
OTERYN_DATABASE_URL=postgres://... cargo run -p oteryn-portal -- migrate
OTERYN_DATABASE_URL=postgres://... cargo run -p oteryn-portal -- serve
```

## Rules for every task

- One task changes one module crate (`crates/<module>`), plus its own migration file, templates under `templates/<module>/` and one line where its router is merged in `crates/web`.
- Migrations are additive: `migrations/<UTC timestamp>_<module>_<what>.sql`. Never edit a merged migration.
- No `unsafe`, `unwrap`, `expect` or `panic` outside tests. Errors are typed with `thiserror`.
- Every mutation is authorized server-side. Browser mutations need CSRF. Money uses a transaction with a row lock and an idempotency key.
- Game-facing `/internal/v1` routes keep the existing contracts byte for byte; tests read fixtures from `docs/contracts/fixtures/`.
- Features that touch the game or money are off by default behind a flag.
- Never commit secrets. Configuration comes from `OTERYN_*` environment variables.
