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

The pinned upstream procedure is `references/upstream.md`, with trailing whitespace normalized. Its examples are available under this skill directory. The upstream `with_server.py` helper is deliberately not installed: a TCP-port probe can accept an unrelated listener and its subprocess cleanup is incomplete. Ignore reference instructions to run that absent helper; use the repository's existing test launcher and prove application readiness with a real element/response. No downstream server-launcher fork is introduced.

Do not run a browser suite on every edit or mirror implementation with redundant tests. Follow repository-required checks even if this optional skill is not enabled.

The wrapper follows the repository license. The Anthropic examples and normalized reference retain Apache-2.0 under `LICENSE.txt`; pinned provenance is in `provenance.json`.
