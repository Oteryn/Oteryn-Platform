---
name: webapp-testing
description: Optional targeted browser verification of changed Oteryn Platform user flows using Playwright, only after explicit user authorization to use this skill or select relevant optional skills for the current task. Not for native Rust clients, backend-only work or unrelated full-suite runs.
---

# Oteryn Platform browser verification

Check current-task authorization in root AGENTS.md. Without a grant, use normal required checks without applying this optional workflow.

1. Read the applicable build/test matrix and existing browser tests. Reuse the repository's launcher, fixtures and Playwright setup before using bundled examples or writing a new harness.
2. Select the smallest affected actor-to-result flow. Test only an authorized local/test environment with synthetic accounts/data. Skill permission is not production, live-account or deployment permission.
3. Check Python/Playwright/browser availability. Do not silently install dependencies or system tools. If unavailable, report the missing browser evidence and continue checks that can run; do not fabricate a pass.
4. Prefer semantic locators and assertions about visible, actionable UI. Wait for the actual ready element or response. Upstream examples using `networkidle` or fixed waits are not a readiness guarantee for persistent network connections.
5. Check the requested behavior and relevant errors/focus/viewport states. Capture bounded screenshots and browser errors without credentials, cookies or personal data. Record tested scope and limits.

The pinned upstream procedure is `references/upstream.md`. Its example paths are relative to this skill directory. `scripts/with_server.py` is an optional helper, not the default repository launcher: run `--help` first, use only trusted local commands, and never put credentials in its command arguments (it prints commands). It only probes a TCP port and terminates its immediate subprocess; prove application readiness and clean up any descendants with the existing test launcher when applicable. Do not use it for long-running/noisy servers whose piped output could block execution.

Do not run a browser suite on every edit or mirror implementation with redundant tests. Follow repository-required checks even if this optional skill is not enabled.

The wrapper follows the repository license. The unmodified Anthropic examples/helper/reference retain Apache-2.0 under `LICENSE.txt`; pinned provenance is in `provenance.json`.
