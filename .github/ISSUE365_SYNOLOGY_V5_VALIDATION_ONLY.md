# Issue 365 isolated Synology validation v5

Temporary validation-only branch for Issue #727 / parent #365.

- Never merge this branch or its observation PR.
- Consume only artifact `8964153679` from run `31092791643`.
- Keep every orchestration/status file under `RUNNER_TEMP`, outside the frozen checkout.
- Prove the frozen checkout is clean before any validator-status write.
- Invoke the approved validator exactly once, with `set +e` only around that command so its exit code can be retained.
- Workers are `1`; Playwright retries are `0` as statically proven.
- Retain full or partial evidence and cleanup status.
- Do not modify the validator, frozen target, application, dependencies, deployment, production, Cloudflare, Canary or external repositories.
- Close the observation PR after terminal classification.
