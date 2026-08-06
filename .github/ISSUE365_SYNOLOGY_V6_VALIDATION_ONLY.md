# Issue 365 complete-contract Synology validation v6

Temporary validation-only branch for Issue #735 / parent #365.

- Never merge this branch or its observation PR.
- Consume only validator artifact `8964153679` and environment proof artifact `8964791387`.
- Verify both metadata records and all internal hashes.
- Require the environment manifest to match the exact workflow values and `unresolved=[]`.
- Keep orchestration state under `RUNNER_TEMP` and prove the frozen checkout clean before execution.
- Invoke the approved validator exactly once with the complete contract.
- Retain full or partial evidence and cleanup status.
- Do not modify the validator, frozen target, application, dependencies, deployment, production, Cloudflare, Canary or external repositories.
- Close the observation PR after terminal classification.
