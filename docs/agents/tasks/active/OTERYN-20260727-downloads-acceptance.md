---
task_id: OTERYN-20260727-downloads-acceptance
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0015-machine-enforced-portal-acceptance-ledger.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
search_first:
  - open pull requests and active tasks owning Downloads, localization or acceptance paths
  - existing Download feature, URL-policy, localization and administrator tests
  - current Download routes, controllers and views
optional_reads:
  - docs/architecture/SECURITY_ARCHITECTURE.md
---

# OTERYN-20260727-downloads-acceptance

## Goal

Close the `downloads.public-admin-localization` ledger record through a bounded composed browser lifecycle while preserving the existing URL allowlist, immutable publication, MFA and exact-permission contracts.

## Acceptance criteria

- [ ] Public Download Center proves empty, current release, platform filtering, approved metadata and unavailable/fail-closed states.
- [ ] Administrator lifecycle proves guest, no-MFA, no-permission and authorized create/publish boundaries.
- [ ] Browser validation rejects an unapproved executable URL and never uploads, fetches or proxies executable content.
- [ ] English and Polish public behavior proves no English release-note fallback and a published Polish translation.
- [ ] Desktop, tablet and mobile layouts have no horizontal page overflow; scrollable tables remain usable.
- [ ] The Downloads ledger record becomes `covered` only after stable exact evidence exists.
- [ ] Target Downloads browser execution, route classification, account lifecycle and all required repository checks pass on the exact final head.

## Ownership

```yaml
owned_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - scripts/acceptance/package.json
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-exhaustive-portal-acceptance.md
modules:
  - Downloads
  - Localization
  - Testing / Acceptance E2E
  - Agent governance
dependencies:
  - PR 247 merged as 4e8a11a9b76aeaaa59a5dcc38bcd8a8e2fa54b39
  - Issue 240
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T19:00:00+02:00
head: 4e8a11a9b76aeaaa59a5dcc38bcd8a8e2fa54b39
branch: test/OTERYN-20260727-downloads-acceptance
pr: none
status: implementing
context_routes:
  - agent-governance
  - testing
  - web-cms
  - admin-rbac
  - security
  - accessibility
owned_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - scripts/acceptance/package.json
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-exhaustive-portal-acceptance.md
proven:
  - PR 247 introduced the canonical ledger and records Downloads as planned
  - deterministic feature tests already prove current-only publication, URL-policy rejection, MFA, exact permission, immutable publication and Polish no-fallback behavior
  - no open pull request owns Downloads product or acceptance paths
derived:
  - the remaining gap is composed browser evidence rather than a known Downloads runtime defect
unknown:
  - exact administrator translation-form labels and any browser-only usability defect
conflicts: []
first_failure:
  marker: none
  evidence: no Downloads browser package has run on this branch yet
rejected_hypotheses:
  - existing feature tests alone prove the composed public and administrator browser lifecycle
  - executable upload or proxy behavior should be added to make the Download Center complete
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260727-downloads-acceptance.md
validation:
  - command: repository and open-PR inspection
    result: PASS
    evidence: Downloads implementation, lower-layer tests, ledger gap and non-overlapping active PRs inspected
blockers:
  - none
next_action: Open a draft PR, implement the bounded Downloads browser lifecycle and run its exact-head workflow.
```

## Notes

This package closes only the delivered Download Center contract. It does not add executable hosting, production deployment or `PRODUCTION_PROVEN` evidence.
