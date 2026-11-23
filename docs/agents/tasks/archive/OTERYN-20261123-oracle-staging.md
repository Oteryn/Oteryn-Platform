---
task_id: OTERYN-20261123-oracle-staging
governing_issue: 2047
required_reads:
  - deploy/oracle/README.md
  - deploy/oracle/actions-deploy.py
  - deploy/oracle/portal.Dockerfile
  - portal-rs/e2e/ENVIRONMENT.md
  - portal-rs/README.md
search_first: []
optional_reads: []
---

# OTERYN-20261123-oracle-staging

## Goal

Governing GitHub Issue [#2047](https://github.com/Oteryn/Oteryn-Platform/issues/2047),
programme [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002).

Let package operators run an eligible branch head on Oracle through a root-host
CLI with a disposable database, browser results and optional PostgreSQL crate
tests. The owner's updated design uses direct host operation.

## Acceptance criteria

- [x] Validate the CLI and resolve the exact current head of an eligible branch.
- [x] Detach the deploy and record progress, readiness and compact test results.
- [x] Isolate staging state, Docker project, runtime network and Cargo caches.
- [x] Check the production lock and pending receipts with read-only access.
- [x] Recreate, migrate and seed PostgreSQL and provide an authenticated mail catcher.
- [x] Run pinned Playwright with an optional literal file filter.
- [x] Run requested crate tests against independent disposable databases.
- [x] Retain recent staging runs and document owner installation and operation.

## Ownership

```yaml
owned_paths:
  - deploy/oracle/staging-deploy.py
  - deploy/oracle/staging.compose.yml
  - deploy/oracle/staging.Dockerfile
  - deploy/oracle/test_staging_deploy.py
  - deploy/oracle/README.md
  - docs/agents/tasks/active/OTERYN-20261123-oracle-staging.md
modules:
  - Oracle staging
dependencies:
  - Oteryn Platform issue 2047
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-23T08:40:00Z
head: 10338ae9d805de86cccacf1ef753e2376c3afebf
branch: codex/oracle-staging
pr: 2050
status: completed
context_routes:
  - Oracle deployment and portal testing
owned_paths:
  - deploy/oracle/staging-deploy.py
  - deploy/oracle/staging.compose.yml
  - deploy/oracle/staging.Dockerfile
  - deploy/oracle/test_staging_deploy.py
  - deploy/oracle/README.md
  - docs/agents/tasks/active/OTERYN-20261123-oracle-staging.md
proven:
  - Local Docker started the pinned database and mail catcher with the staged TLS configuration.
  - Authenticated SMTP negotiated STARTTLS with the generated CA.
  - The distinct PostgreSQL test role could create fixture databases and was denied access to the portal database.
derived:
  - The installed helper keeps build recipe, runtime Compose and branch sources under separate ownership.
unknown:
  - The owner will qualify a real branch deploy on the Oracle ARM64 host after installation.
  - The local Playwright container check could not start Chromium with the available Docker disk.
conflicts: []
first_failure:
  marker: staging CLI boundary tests on base
  evidence: The new tests initially failed because the staging helper was absent.
rejected_hypotheses: []
changed_paths:
  - deploy/oracle/staging-deploy.py
  - deploy/oracle/staging.compose.yml
  - deploy/oracle/staging.Dockerfile
  - deploy/oracle/test_staging_deploy.py
  - deploy/oracle/README.md
  - docs/agents/tasks/active/OTERYN-20261123-oracle-staging.md
validation:
  - command: python3 -m unittest discover -s deploy/oracle -p 'test_*.py'
    result: PASS
    evidence: 104 tests passed with the pinned Caddy fixture, including 18 new staging tests.
  - command: python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint
    result: PASS
    evidence: Validated 27 active task packets.
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: The legacy-token ratchet passed on the staging change.
blockers:
  - none
next_action: none; delivered by PR #2050 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Dedicated same-repository task branch for issue 2047.
source_branch_evidence: PR #2050 carries codex/oracle-staging; the branch is deleted when it merges. The files installed on Oracle match 10338ae9 by sha256. A live redeploy passed security/auth-csrf.spec.mjs 4/4 in 48 s.
```

## Notes

The checkpoint head records the branch parent at the time the packet was written.
Oracle host installation and live staging qualification are owner operations.
