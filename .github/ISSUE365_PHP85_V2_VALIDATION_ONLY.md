# Issue 365 PHP 8.5 validation v2

Temporary harness-only branch for Issue #717 / parent #365.

- Never merge this branch or its observation PR.
- Phase 1 must reconstruct and statically validate the exact generated script on `ubuntu-latest`.
- The Synology matrix job may start only after phase-1 PASS.
- At most one Synology matrix job is authorized.
- Workers are `1`; Playwright retries are `0`.
- No application, dependency-lock, deployment, production, Cloudflare, Canary or external-repository mutation is allowed.
- Technical failures are not product evidence.
- Close the observation PR after the terminal result is recorded.
