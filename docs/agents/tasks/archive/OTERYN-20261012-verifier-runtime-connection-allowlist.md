---
task_id: OTERYN-20261012-verifier-runtime-connection-allowlist
governing_issue: none
required_reads: []
search_first:
  - app/Operations/ProductionConfigurationVerifier.php
optional_reads: []
---

# OTERYN-20261012 verifier runtime connection allowlist

## Goal

Review finding on #1577/#1579, real on main: `ProductionConfigurationVerifier` accepted any default connection whose driver was pgsql or mysql, including `pgsql_migrations` (schema-owner credentials) and `canary_character_transfer`. The verifier now also requires the default connection name to be a runtime connection (`pgsql`, `mysql`, `mariadb`) and refuses migration, owner, backup, dump and Canary connections with a clear message. No governing GitHub Issue; the review finding is the authority.

## Acceptance criteria

- [x] Default connection must be on the runtime allowlist; other server-driver connections are refused.
- [x] Unit tests cover acceptance (pgsql, mysql, mariadb) and refusal of pgsql_migrations, canary_character_transfer, canary and owner/backup/dump-style names.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - app/Operations/ProductionConfigurationVerifier.php
  - tests/Feature/Operations/ProductionConfigurationVerifierTest.php
  - docs/agents/tasks/archive/OTERYN-20261012-verifier-runtime-connection-allowlist.md
modules:
  - Operations
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T00:00:00Z
head: UNKNOWN
branch: claude/platform-verifier-allowlist-20261012
pr: UNKNOWN
status: validating
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - app/Operations/ProductionConfigurationVerifier.php
  - tests/Feature/Operations/ProductionConfigurationVerifierTest.php
proven:
  - config/database.php has runtime connections pgsql and mysql and privileged pgsql_migrations, canary, canary_character_transfer with server drivers
derived:
  - allowlist is checked after the driver check so existing sqlite and unknown-connection messages are unchanged
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/Operations/ProductionConfigurationVerifier.php
  - tests/Feature/Operations/ProductionConfigurationVerifierTest.php
validation:
  - command: vendor/bin/pint --test on changed PHP files
    result: PASS
    evidence: local
  - command: phpunit ProductionConfigurationVerifierTest, phpstan
    result: BLOCKED
    evidence: local PHP 8.3.6 but dependencies require PHP >= 8.4.1; exact-head CI is authoritative
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: production configuration verifier hardening, no product path
blockers:
  - none
next_action: none; record archived in the authoring commit
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: pending
```

## Notes

Allowlist is a constant of runtime names rather than derived by exclusion, so a newly added privileged connection is refused by default.
