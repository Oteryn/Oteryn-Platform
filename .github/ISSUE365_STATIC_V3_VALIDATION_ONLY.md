# Issue 365 static validator proof v3

Temporary static-only branch for Issue #721 / parent #365.

- Never merge this branch or its observation PR.
- Run only on GitHub-hosted `ubuntu-latest`.
- Do not allocate a self-hosted or Synology runner.
- Reconstruct the exact immutable generator inputs and prove the generated Bash validator statically.
- Do not run Docker, databases, Redis, MailHog, Playwright browsers or product samples.
- Close the observation PR after the single terminal static workflow result is recorded.
