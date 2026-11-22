# Portal operator CLI end-to-end tests

Install the locked Node dependencies from `portal-rs/e2e` with `npm ci`.
Start the local PostgreSQL service described in the [E2E harness README](../README.md)
and set `PORTAL_E2E_ADMIN_DATABASE_URL` in your environment to its local database
administrator PostgreSQL URL with an explicit database path. Use the URL authority
for host and port and omit query parameters. Run from `portal-rs/e2e`.

```sh
CARGO_BUILD_JOBS=2 node --test cli/*.test.mjs
```

The runner starts `harness.mjs`, waits for its readiness report, and uses its
disposable PostgreSQL database. It stops the harness and removes private scratch
files at teardown. Set `PORTAL_E2E_BINARY` to an absolute path to a prebuilt
`oteryn-portal` to use that executable. The standard harness listener overrides
apply if its default ports are in use.

The suite covers `migrate`, the seed entities, `bootstrap-intent issue`,
`catalog import`, `catalog activate`, `game-route publish`,
`commerce-refunds-reconcile`, `downloads-publish`, and `check-image`.
Seed and route publication failures use their documented stdout JSON envelopes.
Download publication refuses an exact replay of signed metadata and retains the
active release. MFA enrollment refuses a repeat and withholds the original
enrollment material. The native-evidence seed reports that evidence can only
be observed live.
