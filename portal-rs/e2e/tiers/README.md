# Browser CI tiers

The smoke manifest assigns the `@smoke` tag outside the specs. The runner
escapes the titles for Playwright's `--grep` and checks discovery for exactly
one match per title and project. The registration journey covers both the
registration form render and successful login. The staff case verifies the
landing page before creating a role and checking its audit record.

For a local run, follow the Node, Chromium and disposable PostgreSQL setup in
[the harness instructions](../README.md), then run these commands from
`portal-rs/e2e`:

```sh
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts --cache /tmp/portal-browser-npm
CHROMIUM_PATH=/usr/bin/chromium node tiers/run.mjs smoke
```

The tier runner starts and stops `local.mjs` and invokes `run.mjs` once the HTTPS
probe succeeds. Set `PORTAL_E2E_BINARY` to use a binary built from the same
checkout. The smoke job has an 11-minute timeout after the shared binary build.

The nightly matrix runs three file shards for each of the four projects.
The first shard keeps the staff bootstrap together with the content, routes
and staff specs that consume its browser storage state. The other two shards
split the remaining sorted spec filenames. Each shard gets separate databases
and browser state. Its HTML artifact includes source locations and outcomes.
Raw browser output and harness logs stay in the private scratch directory.

The registered `portal-rs-e2e.yml` workflow runs smoke for PRs and the full
matrix on schedule or dispatch. Deploy requires the newest scheduled run on
`main` for the current nightly window to complete successfully. Each window
starts at 02:17 UTC. Manual runs can qualify the suite on demand.

Add the PR coverage gate when the ledger lands, with its entry point and
committed baseline both required.
