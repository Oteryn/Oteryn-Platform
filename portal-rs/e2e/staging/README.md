# Staging browser tests

This suite uses Chromium against https://staging.oteryn.com. Set
`STAGING_CF_ACCESS_CLIENT_ID` and `STAGING_CF_ACCESS_CLIENT_SECRET` in the
environment before running it. The local Playwright package and Chromium at
`/usr/bin/chromium` are required.

From the repository root:

```sh
(cd portal-rs/e2e && PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts)
npx --prefix portal-rs/e2e playwright test -c portal-rs/e2e/staging/playwright.config.mjs
```

The four projects use Polish or English at 1440 or 390 pixels. Tests that need
an account register a unique `@example.test` address through the public form.
Public routes and malformed-form tests can run without creating an account.
The suite checks `/readyz` before each case. Two workers share the registration
request budget. An unexpected 429 can come from other activity on the staging
IP. Wait for its rate-limit window to clear, then rerun the affected case with
`--project` and `--grep`. The suite has zero automatic test retries. It logs a
single retry of an explicit GET browser navigation on a network failure or 502.

The suite checks Mailpit on startup. A 404 or 405 skips the `@mail` cases and
leaves the other cases running. When tracking a Mailpit rollout, recheck at
ten-minute intervals for up to an hour. Once the endpoint returns 200, run
the same command with `--grep @mail`. Mail reads use GET and match the test
recipient and localized subject during a 60-second poll. Test 3.3 waits
61 minutes for an unused reset link to expire on the server. It can be run
by itself with `--grep @expiry`.

Access headers are attached to staging requests only. Chromium uses an
origin-restricted proxy configuration to prevent forwarding the headers
during browser redirects. When `HTTPS_PROXY` and `CODEX_PROXY_CERT` are set,
Chromium pins the execution proxy's CA public key. Traces, video and HAR
recording are disabled. The console reporter prints case statuses and
sanitized failures. Failure screenshots with sensitive fields masked are
written to a run-specific `oteryn-staging-blackbox-*` directory under the
system temporary directory.
