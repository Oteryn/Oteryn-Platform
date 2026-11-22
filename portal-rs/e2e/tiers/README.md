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
checkout. Each smoke project gets a separate 11-minute job after the shared
binary build. Pass a project name after `smoke` to run that project locally.

The nightly matrix discovers the configured Playwright projects and runs three
file shards per project. The tier config preserves the browser suite's test
patterns, including legacy Bazaar and authentication security cases when
configured. For each project, the runner discovers all cases, lists each shard,
and checks that the combined case identities equal the full discovery exactly
once. It checks the executed cases against the selected shard as well.
The first shard keeps the staff bootstrap with the Bazaar moderation, content,
routes and staff specs that consume its browser storage state. The other two shards
split the remaining sorted spec filenames. Each shard gets separate databases
and browser state. Its HTML artifact includes case IDs, catalogued route labels, source locations,
outcomes and fixed failure descriptions.
Raw browser output and harness logs stay in the private scratch directory.

The registered `portal-rs-e2e.yml` workflow runs smoke for PRs and the full
matrix on schedule or dispatch. Deploy requires the newest scheduled run on
`main` for the current nightly window to complete successfully. Each window
starts at 02:17 UTC. The root-owned Oracle deploy wrapper checks the window
again immediately before migration. The workflow sends `deploy-v2`. Install
the matching wrapper through the manual Oracle installation procedure before
enabling deploy.
Manual runs can qualify the suite on demand. Only the newest scheduled run on
`main` can update the failure issue.

## Rollout steps for the owner

1. Create the `e2e-nightly` label in the Platform repository.
2. Provision a dedicated issue titled `Nightly E2E failing` with that label.
   Put `<!-- oteryn:portal-rs-e2e-nightly -->` on the first line of its body
   to opt the issue into automated updates.
3. Set the repository Actions variable `PORTAL_E2E_NIGHTLY_ISSUE_NUMBER` to
   that issue number before enabling the deployment gate. The tracker fails
   with a setup message for an unset or invalid number. A pinned issue whose
   identity checks fail also makes the scheduled run ineligible for deploy.

Add the PR coverage gate when the ledger lands, with its entry point and
committed baseline both required.
