---
task_id: OTERYN-20261012-platform-dec-2
governing_issue: 1474
required_reads: []
search_first:
  - docs/agents/LEGACY_TOKEN_BASELINE.json
optional_reads: []
---

# OTERYN-20261012 docs-sync-1 and Redis and ticket-field cleanup

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

After #1556 retired the runtime Redis path: (1) mark the legacy runtime Redis boundary as historical in ROADMAP, MODULE_CATALOG and PRODUCTION_TOPOLOGY_EVIDENCE; (2) remove the unused Redis service containers from acceptance workflows; (3) stop writing, checking and prohibiting the legacy ticket account column on native login tickets. No migration and no data drop; the database column stays until a later migration.

## Acceptance criteria

- [x] Three docs no longer present the legacy runtime Redis boundary as current.
- [x] Redis service removed from 12 workflows; kept only in `acceptance-validation.yml`, whose visual and resilience scripts still stop and restore Redis.
- [x] the legacy ticket account column removed from ticket issue, redemption check, model and controller rule; tests updated.
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - docs/architecture/ROADMAP.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/operations/PRODUCTION_TOPOLOGY_EVIDENCE.md
  - .github/workflows
  - app/GameAuth
  - app/Http/Controllers/GameAuth/GameLoginTicketIssueController.php
  - tests/Feature/GameAuth
  - docs/agents/tasks/active/OTERYN-20261012-platform-dec-2.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-dec-2.md
modules:
  - game-auth
  - ci
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
updated_at: 2026-10-12T08:00:00Z
head: UNKNOWN
branch: claude/platform-decanary-2-20261012
pr: 1557
status: validating
context_routes:
  - ci-workflows
owned_paths:
  - docs/architecture/ROADMAP.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/operations/PRODUCTION_TOPOLOGY_EVIDENCE.md
  - .github/workflows
  - app/GameAuth
  - app/Http/Controllers/GameAuth/GameLoginTicketIssueController.php
  - tests/Feature/GameAuth
  - docs/agents/tasks/active/OTERYN-20261012-platform-dec-2.md
proven:
  - app and config contain no Redis client use after #1556
  - acceptance-validation scripts still use redis-cli and the resilience spec
derived:
  - other acceptance workflows set file or array cache and session drivers, so they need no Redis service
unknown:
  - PHPUnit result until CI runs on this head
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/architecture/ROADMAP.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/operations/PRODUCTION_TOPOLOGY_EVIDENCE.md
  - .github/workflows
  - app/GameAuth/NativeLogin/NativeGameLoginTickets.php
  - app/GameAuth/Tickets/GameLoginTicket.php
  - app/Http/Controllers/GameAuth/GameLoginTicketIssueController.php
  - tests/Feature/GameAuth
  - docs/agents/tasks/active/OTERYN-20261012-platform-dec-2.md
validation:
  - command: legacy_token_ratchet, checkpoint, documentation_ia, php -l, pytest tests/ci
    result: PASS
    evidence: local run
blockers:
  - none
next_action: Archive this record once the PR is closed.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: pending
```

## Notes

Follow-up: drop `game_login_tickets.legacy ticket account column` in a later E2 migration; decide whether the Redis fault-injection in acceptance-validation can be retired.
