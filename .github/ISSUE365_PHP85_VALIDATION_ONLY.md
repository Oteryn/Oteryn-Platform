# Issue 365 PHP 8.5 validation branch

This branch is a temporary, harness-only observation channel for Issue #712 / parent #365.

- Do not merge.
- Do not deploy.
- Do not change application, route, view, migration, dependency-lock or production state.
- Execute at most one bounded Synology matrix run with workers `1` and retries `0`.
- Classify technical failures separately from product evidence.
- Preserve partial evidence when the matrix exits or is cancelled.
- Close the observation pull request after the terminal result is recorded.
