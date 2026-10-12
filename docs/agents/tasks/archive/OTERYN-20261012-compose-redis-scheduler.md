---
task_id: OTERYN-20261012-compose-redis-scheduler
governing_issue: 1474
required_reads: []
search_first:
  - deploy/synology/compose.yml
optional_reads: []
---

# OTERYN-20261012 Synology Compose: remove Redis, scheduler always on

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Platform review #1560 items D-4 and D-6, in force by owner mandate. (D-4) Remove the unused Redis service, the Platform dependency on it, its volume and `REDIS_PASSWORD` from the Synology staging Compose, environment example, deploy scripts and their tests. (D-6) Make the scheduler an always-on base service in `compose.yml` instead of a Marketplace-only overlay service.

## Acceptance criteria

- [x] `compose.yml` has no Redis service, `depends_on` entry, `redis_data` volume or `REDIS_PASSWORD`; session, cache and queue stay `file`, `file`, `sync` and no config defaults to Redis (`config/cache.php`, `config/queue.php`, `config/session.php`).
- [x] `.env.example`, `deploy.sh`, `deploy-impact.sh`, `health-check.sh` and `production-target-preflight.sh` no longer require, check or preserve Redis.
- [x] The scheduler (`php artisan schedule:work`, Platform image, `restart: always`, healthcheck, no published port) is a base service; `compose.marketplace.yml` only layers the public `APP_URL` and `MARKETPLACE_ENABLED` onto it.
- [x] Every deploy route recreates the scheduler with the selected Platform image (`lib.sh` reconcile); the Marketplace disable and rollback routes keep it running with the switch off; `health-check.sh` requires it healthy.
- [x] Tests updated and a Compose runtime contract added.
- [x] Seven workflows, the README and the Dockerfile no longer name Redis (owned-path extension by the lead); the unused phpredis extension is dropped; preflight restart policy is `always`, matching Compose.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - deploy/synology/compose.yml
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/.env.example
  - deploy/synology/scripts/**
  - deploy/synology/tests/**
  - tests/ci/test_synology_*.py
  - .github/workflows/repair-synology-autostart.yml
  - .github/workflows/repair-synology-compose-orphans.yml
  - .github/workflows/synology-diagnostics.yml
  - .github/workflows/synology-container-hygiene.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - deploy/synology/README.md
  - deploy/synology/docker/platform.Dockerfile
  - docs/agents/tasks/archive/OTERYN-20261012-compose-redis-scheduler.md
modules:
  - deploy-synology
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
updated_at: 2026-10-12T08:40:00Z
head: see PR head
branch: claude/compose-redis-scheduler-20261012
pr: 1566
status: completed
context_routes:
  - deploy
owned_paths:
  - deploy/synology/compose.yml
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/.env.example
  - deploy/synology/scripts/**
  - deploy/synology/tests/**
  - tests/ci/test_synology_*.py
  - .github/workflows/repair-synology-autostart.yml
  - .github/workflows/repair-synology-compose-orphans.yml
  - .github/workflows/synology-diagnostics.yml
  - .github/workflows/synology-container-hygiene.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - deploy/synology/README.md
  - deploy/synology/docker/platform.Dockerfile
  - docs/agents/tasks/archive/OTERYN-20261012-compose-redis-scheduler.md
proven:
  - Rendered Compose (base, and base plus marketplace overlay) has services mariadb, platform, tls-init, internal-proxy, gateway and marketplace-scheduler, no redis, and a scheduler with schedule:work, restart always, healthcheck, public APP_URL and MARKETPLACE_ENABLED false by default.
  - Cache, session and queue drivers default to file, file and sync in config and are set the same in Compose.
derived:
  - The healthcheck reads /proc/1/cmdline because the entrypoint ends in exec, so PID 1 is the schedule:work process; it also opens a PDO connection to MariaDB.
unknown:
  - The scheduler healthcheck has not run against a real container in this task; the first staging deploy proves it.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Rename the service to scheduler. The hygiene and Character Bazaar workflows address marketplace-scheduler by name and are not owned here.
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - .github/workflows/repair-synology-compose-orphans.yml
  - .github/workflows/synology-diagnostics.yml
  - .github/workflows/synology-container-hygiene.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - deploy/synology/README.md
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/compose.yml
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/.env.example
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/marketplace-staging.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - tests/ci/test_synology_autostart_contract.py
  - tests/ci/test_synology_compose_runtime_contract.py
  - tests/ci/test_synology_native_admission_gate.py
  - tests/ci/test_synology_native_admission_trust_publication.py
  - tests/ci/test_synology_rollback_contract.py
  - .github/workflows/repair-synology-autostart.yml
  - .github/workflows/repair-synology-compose-orphans.yml
  - .github/workflows/synology-diagnostics.yml
  - .github/workflows/synology-container-hygiene.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - deploy/synology/README.md
  - deploy/synology/docker/platform.Dockerfile
  - docs/agents/tasks/archive/OTERYN-20261012-compose-redis-scheduler.md
validation:
  - command: python3 -m pytest tests/ci tools/validation deploy/synology/tests
    result: PASS
    evidence: 477 passed
  - command: docker compose config (base and base plus marketplace overlay)
    result: PASS
    evidence: renders; scheduler has no published port
  - command: python3 tools/validation/legacy_token_ratchet.py; workflow_inventory.py; github_actions_pinning.py
    result: PASS
    evidence: baseline not raised; 53 workflows
  - command: python3 -m pytest tools/agents
    result: PASS
    evidence: 14 errors are identical on unmodified main (pre-existing, fixture errors), none from this change
blockers:
  - none
next_action: none after merge. Renaming marketplace-scheduler to scheduler is a separate later task.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository feature branch with no retention purpose after merge
source_branch_evidence: same-repository PR #1566 from this branch; the repository setting delete_branch_on_merge is true, so a merge removes the ref; confirm it is absent after merge
```
