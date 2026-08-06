# Issue 365 one-shot Synology validation v4

Temporary validation-only branch for Issue #724 / parent #365.

- Never merge this branch or its observation PR.
- Consume only artifact `8964153679` from run `31092791643`.
- Verify GitHub artifact metadata and internal `SHA256SUMS` before execution.
- Execute exactly one self-hosted job and one generated validator invocation.
- Workers are `1`; Playwright retries are `0` as statically proven.
- Retain full or partial evidence and runtime status even on failure.
- Do not modify the validator, frozen target, application, dependencies, deployment, production, Cloudflare, Canary or external repositories.
- Close the observation PR after the terminal result is recorded.
