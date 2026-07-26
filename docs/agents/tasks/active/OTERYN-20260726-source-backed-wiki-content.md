---
task_id: OTERYN-20260726-source-backed-wiki-content
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0005-character-creation-product-policy.md
  - docs/architecture/adr/0010-wiki-module-and-persistence-foundation.md
  - docs/architecture/adr/0012-public-wiki-read-search.md
search_first:
  - Issue 145 minimum launch-content checklist versus trusted main
  - active tasks and open pull requests with overlapping Wiki, console-command, test or ADR paths
  - approved repository sources for every launch-content claim
  - existing Wiki article, category, lifecycle, permission, audit and revision services
  - existing idempotent operator-command and content-provisioning conventions
  - current browser acceptance Wiki fixtures and coverage
optional_reads: []
---

# OTERYN-20260726-source-backed-wiki-content

## Goal

Provide the minimum bilingual Wiki launch-content set through a reviewable, fail-closed and idempotent operator workflow, using only claims proven by accepted repository policy or current Platform behavior and never inventing gameplay facts.

## Acceptance criteria

- [ ] The thirteen minimum launch topics in `WIKI_IMPLEMENTATION_PLAN.md` have reviewed English and Polish Wiki articles.
- [ ] Every substantive product or gameplay claim is traceable to an accepted ADR, current route/service/UI contract or an explicit public dependency state.
- [ ] Unknown server rates, detailed vocation mechanics, current PvP rules, Discord destination and authoritative game-client login rollout are not guessed.
- [ ] Installation requires one named existing MFA-confirmed publisher with all exact Wiki permissions; no wildcard or implicit authority is granted.
- [ ] Provisioning uses existing Wiki services so category/article authorization, revisions, lifecycle, audit and publication invariants remain authoritative.
- [ ] Provisioning is atomic, idempotent on an exact installed set and fail-closed on slug/key/content conflicts without overwriting editorial changes.
- [ ] Focused command, permission, rollback, idempotency, content-source and public EN/PL coverage passes.
- [ ] Formatter, static analysis, full required repository validation and browser acceptance pass on the exact final head.

## Ownership

```yaml
owned_paths:
  - app/Console/Commands/InstallWikiLaunchContent.php
  - app/Wiki/Content/**
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - tests/Unit/Wiki/WikiLaunchContentCatalogTest.php
  - docs/content/WIKI_LAUNCH_CONTENT_SOURCES.md
  - docs/architecture/adr/0015-wiki-launch-content-provisioning.md
  - docs/architecture/adr/README.md
  - docs/agents/tasks/active/OTERYN-20260726-source-backed-wiki-content.md
modules:
  - Wiki
  - Admin/RBAC
  - Audit
  - Database
  - Testing
dependencies:
  - Issue 145
  - merged PRs 158, 194, 196, 199 and 206
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T13:45:00Z
head: e0b4291b30dcca088967b1b4f307b6bab87a2fcd
branch: codex/OTERYN-20260726-source-backed-wiki-content
pr: none
status: investigating
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - database
  - admin-rbac
  - security
  - testing
owned_paths:
  - app/Console/Commands/InstallWikiLaunchContent.php
  - app/Wiki/Content/**
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - tests/Unit/Wiki/WikiLaunchContentCatalogTest.php
  - docs/content/WIKI_LAUNCH_CONTENT_SOURCES.md
  - docs/architecture/adr/0015-wiki-launch-content-provisioning.md
  - docs/architecture/adr/README.md
  - docs/agents/tasks/active/OTERYN-20260726-source-backed-wiki-content.md
proven:
  - trusted main is e0b4291b30dcca088967b1b4f307b6bab87a2fcd and the local worktree was clean before branching
  - Issue 145 remains open and identifies initial Wiki content plus final exact-SHA staging acceptance as the only remaining programme work
  - WIKI_IMPLEMENTATION_PLAN.md requires thirteen bilingual minimum launch topics and forbids promoting unknowns into assumptions
  - current open pull requests modify only Liquid20 deployment evidence or scheduled-E2E documentation and do not overlap this task
  - PR 158 merged the Wiki foundation; its still-active task record is a stale ready-state advisory lock, not a live open implementation owner
  - existing Wiki application services enforce exact permissions, transactions, append-only revisions, optimistic locking, bounded audit and bilingual publication
  - accepted ADR 0005 proves the five available creation-time vocation choices and character creation policy
  - the current Platform provides real Download, registration, account, character creation, MFA, server information, rules, support and report-a-bug routes
derived:
  - a named operator command using the existing Wiki services can provision reviewed content without weakening administrator HTTP boundaries
  - unknown rates, detailed gameplay mechanics, PvP policy, Discord destination and final game-client login behavior must be represented as explicit authority pointers or unavailable facts
unknown:
  - exact current Oteryn server-rate values
  - detailed approved vocation gameplay descriptions
  - exact current Oteryn PvP and game-rule text
  - approved Discord destination
  - completed authoritative game-client login rollout behavior
conflicts: []
first_failure:
  marker: none
  evidence: startup reconciliation found no live path owner or source conflict
rejected_hypotheses:
  - the minimum launch set may contain guessed gameplay defaults: repository governance and the Wiki plan explicitly forbid this
  - the stale Wiki-foundation task is an active concurrent editor: PR 158 is merged and no corresponding open PR exists
  - a data migration should silently publish editorial content: mutable reviewed content requires an explicit operator workflow
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260726-source-backed-wiki-content.md
validation:
  - command: repository, Issue 145, active-task and open-PR reconciliation
    result: PASS
    evidence: exact trusted main, Issue 145 comments, three unrelated open PRs and merged PR 158
blockers:
  - none
next_action: Commit and open the task-only draft PR, then implement the source catalog and atomic exact-permission provisioning command.
```

## Notes

This task does not add new Wiki schema, upload formats, media consumers, permissions, public routes, Canary/login-server behavior, production actions or external-repository writes.
