---
task_id: OTERYN-20260726-trusted-proxy-test-isolation
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/agents/tasks/archive/OTERYN-20260724-trusted-reverse-proxy-scheme.md
search_first:
  - active tasks and open pull requests for TrustedProxySchemeTest or overlapping owned paths
  - repository environment mutation helpers and Laravel Env repository behavior
  - existing trusted-proxy middleware and regression coverage
optional_reads: []
---

# OTERYN-20260726-trusted-proxy-test-isolation

## Goal

Make the existing trusted reverse-proxy regression deterministic when it runs after other Laravel feature tests, without changing the production trust boundary or weakening its assertions.

## Acceptance criteria

- [x] The trusted-proxy regression passes both alone and after another feature test in the same PHPUnit process.
- [x] The complete PHPUnit suite passes with the original HTTPS and untrusted-spoofing assertions intact.
- [x] Formatter, static analysis, focused tests and required CI pass on the exact final head.

## Ownership

```yaml
owned_paths:
  - tests/Feature/Security/TrustedProxySchemeTest.php
  - docs/agents/tasks/active/OTERYN-20260726-trusted-proxy-test-isolation.md
modules:
  - testing
  - security
dependencies:
  - OTERYN-20260724-trusted-reverse-proxy-scheme
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T11:26:20Z
head: 5dbfb1338e1d952c704cb7b30b3d2c00c1164795
branch: fix/OTERYN-20260726-trusted-proxy-test-isolation
pr: 203
status: ready
context_routes:
  - testing
  - security
owned_paths:
  - tests/Feature/Security/TrustedProxySchemeTest.php
  - docs/agents/tasks/active/OTERYN-20260726-trusted-proxy-test-isolation.md
proven:
  - The unchanged main test passes alone but fails after PublicSiteShellTest because Laravel's already-instantiated immutable environment repository does not observe later direct process-global mutation
  - The full Wiki branch suite first fails at TrustedProxySchemeTest with a localhost HTTP form action while every Wiki integration regression passes
  - No active task or open pull request owns TrustedProxySchemeTest or an overlapping test-isolation repair
  - Per-application http.trusted_proxies configuration makes the proxy test pass both alone and after PublicSiteShellTest while preserving all original assertions, including when a local .env already defines the variable
  - Draft PR 203 targets blakinio/Oteryn-Platform main from the dedicated repair branch
derived:
  - The regression should inject the trusted CIDR at the middleware's per-application configuration boundary instead of mutating process environment state
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - PublicSiteShellTest leaks URL generator state: the failure is the trusted proxy configuration missing at bootstrap, and the direct process-global mutation occurs after Laravel caches its Env repository
  - Laravel Env repository mutation is sufficient in every checkout: an existing immutable .env value prevents the test from replacing the repository value
changed_paths:
  - tests/Feature/Security/TrustedProxySchemeTest.php
  - docs/agents/tasks/active/OTERYN-20260726-trusted-proxy-test-isolation.md
validation:
  - command: php artisan test tests/Feature/Security/TrustedProxySchemeTest.php
    result: PASS
    evidence: 2 tests and 6 assertions pass in isolation before repair
  - command: php artisan test tests/Feature/PublicSiteShellTest.php tests/Feature/Security/TrustedProxySchemeTest.php
    result: PASS
    evidence: 5 tests and 19 assertions pass with the final per-application configuration repair and an existing .env
  - command: php artisan test tests/Feature/Security/TrustedProxySchemeTest.php
    result: PASS
    evidence: 2 tests and 6 assertions pass with the final per-application configuration repair and an existing .env
  - command: php vendor/bin/pint --test tests/Feature/Security/TrustedProxySchemeTest.php
    result: PASS
    evidence: one file passes
  - command: php vendor/bin/phpstan analyse tests/Feature/Security/TrustedProxySchemeTest.php --memory-limit=1G --no-progress
    result: PASS
    evidence: no errors
  - command: php artisan test --display-warnings
    result: PASS
    evidence: exact runtime head 5dbfb13 passes 331 tests with 2473 assertions and 10 documented skips
  - command: GitHub required checks on PR 203 head 5dbfb1338e1d952c704cb7b30b3d2c00c1164795
    result: PASS
    evidence: checkpoint-validation, concurrency-proof, test and both validate jobs pass
blockers:
  - none
next_action: Publish this ready checkpoint, verify the docs-only final head checks and squash merge PR 203.
```

## Notes

This prerequisite repair is intentionally separate from Wiki PR #199 because it is an existing test-infrastructure defect on `main`.
