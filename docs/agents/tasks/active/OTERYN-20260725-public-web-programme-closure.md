---
task_id: OTERYN-20260725-public-web-programme-closure
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
search_first:
  - Issue 145 checklist versus current main runtime code
  - active tasks and open pull requests with overlapping paths or intent
  - current public routes, navigation, homepage composition and localization
  - Wiki persistence, services, authorization, routes and tests
  - editorial media consumers and references
  - sitemap, robots, Open Graph and browser acceptance coverage
  - latest trusted main and latest successful Synology staging deployment SHA
optional_reads:
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260725-public-web-programme-closure

## Goal

Coordinate, implement, validate and close the remaining approved non-commercial public website scope tracked by Issue #145 without external-repository, production, router or Internet-exposure writes.

## Acceptance criteria

- [x] Every approved Issue #145 requirement is reconciled against runtime code and exact merged evidence.
- [x] Public Wiki, administration, safe rendering, search and approved media integration are complete in English and Polish.
- [x] Homepage, navigation, SEO, sitemap, robots, responsive and accessibility closure are complete.
- [ ] Approved initial Wiki publication content is present without invented gameplay facts.
- [ ] All required exact-head validation passes.
- [ ] Final trusted `main` is deployed to Synology staging through an existing reviewed workflow and live staging smoke passes.
- [ ] Issue #145 is updated and closed only after its real completion criteria are satisfied.
- [ ] All child tasks are archived and no overlapping implementation PR remains.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
modules:
  - AgentGovernance
  - PublicPortal
  - Wiki
  - EditorialMedia
  - Localization
  - Deployment
  - Testing
dependencies:
  - Issue #145
  - merged PRs #146, #157, #158, #159, #160, #161, #175, #176, #191, #192, #194, #195, #196, #197, #198, #199, #201, #203, #204, #205 and #206
blockers:
  - none for programme coordination
cross_repository_tasks:
  - none
```

The programme task owns coordination only. Implementation paths must be claimed by bounded child tasks and draft PRs before modification.

## Implementation reconciliation

| Requirement | Status | Exact evidence | Remaining action | Proposed owning PR |
|---|---|---|---|---|
| Production homepage and shared public shell | COMPLETE | PRs #146 and #206; `HomePageQuery`; `home.blade.php`; shared header/footer | Preserve without duplicating the shell. | none |
| Dynamic world summary, published news and explicit runtime states | COMPLETE | PRs #146 and #206; `HomePageQuery` and `PublicContentState` | Preserve AVAILABLE/EMPTY/STALE/UNAVAILABLE semantics. | none |
| Announcements and Events modules | COMPLETE | PRs #157 and #206; module code, routes, provider composition and tests | Preserve authoritative providers. | none |
| Announcement ticker and upcoming event on homepage | COMPLETE | PR #206 / `1d063604a66dd3154f97a6f167377d54131cc516`; existing providers composed with truthful states | Preserve. | none |
| Download Center | COMPLETE | PRs #161 and #206; routes, query, views, shared navigation and homepage discoverability | Preserve. | none |
| Server information, beginner guide, support and legal baseline | COMPLETE | PRs #159 and #206; typed editorial routes, queries, navigation and sitemap publication truth | Preserve publication truth and trusted links. | none |
| Guild index | COMPLETE | PRs #160 and #206; route, query, view, shared navigation and acceptance | Preserve. | none |
| PL/EN localization foundation | COMPLETE | PRs #175, #194, #196 and #206; localized shell, Wiki, administration, metadata and browser acceptance | Preserve. | none |
| Wiki persistence, lifecycle, revisions, locking, permissions and audit | COMPLETE | PR #158; Wiki foundation services and reversible persistence | Preserve service and concurrency boundaries. | none |
| Public Wiki routes, categories, articles, breadcrumbs, TOC and related articles | COMPLETE | PR #194 / `9ed3861cc29dcaf6305c379de2bee5ee5ac923d6` | Preserve published-only and locale-freshness constraints. | none |
| Safe Markdown rendering boundary | COMPLETE | PR #194; restricted CommonMark renderer and security regressions | Integrate only approved local media references; do not re-enable remote images. | PR #199 |
| Wiki administration | COMPLETE | PR #196 / `f512f1e3a9bd567d40ddb09b699291c99a1b65f8` | Preserve auth + confirmed MFA + exact permission and optimistic locking. | none |
| Wiki search | COMPLETE | PR #194; published-only locale-isolated bounded search | Preserve no-draft-leak and rate-limit behavior. | none |
| Safe editorial image library foundation | COMPLETE | PR #176; reusable private media boundary | Preserve; do not duplicate storage or upload processing. | none |
| Approved media consumer integration | COMPLETE | PR #199 / `f66c9944fd8110014773bd7cb7b58c9f49e45af0`; ADR 0014; transactional current-translation references; verified published-only delivery; signed administrator previews | Preserve the accepted Wiki-only boundary; Events and CMS remain excluded. | none |
| Canonical and hreflang metadata | COMPLETE | PR #206; shared escaped metadata with freshness-aware equivalent alternates | Preserve. | none |
| Open Graph, sitemap and robots exclusions | COMPLETE | PR #206; bounded OG metadata, fail-closed published-only sitemap and authoritative dynamic robots policy | Preserve. | none |
| Responsive, keyboard and accessibility closure | COMPLETE | PR #206 exact-head critical acceptance plus prior public/admin Wiki coverage | Preserve required profiles. | none |
| Initial approved Wiki content set | MISSING | No approved Wiki publication seed exists | Add only source-backed approved content. | wiki-content child |
| Final Synology staging deployment | BLOCKED | Latest proven staging SHA predates the remaining implementation | Deploy exact final SHA through reviewed workflow and run live smoke after all children merge. | deployment-closure child |
| Issue #145 closure | MISSING | Issue remains open | Update only from merged evidence and close after staging acceptance. | programme closure PR |

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T13:29:00Z
head: 1d063604a66dd3154f97a6f167377d54131cc516
branch: docs/OTERYN-20260726-archive-homepage-navigation-seo
pr: pending archival pull request
status: implementing
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - public-game-data
  - admin-rbac
  - database
  - security
  - testing
  - deployment
  - accessibility
  - localization
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
proven:
  - trusted main is f66c9944fd8110014773bd7cb7b58c9f49e45af0 after merged PR 199
  - Issue 145 remains open
  - PR 194 delivered public published-only English and Polish Wiki reads, restricted Markdown rendering and locale-isolated search as 9ed3861cc29dcaf6305c379de2bee5ee5ac923d6
  - PR 196 delivered trusted Wiki administration as f512f1e3a9bd567d40ddb09b699291c99a1b65f8
  - EditorialMedia reserves the WIKI consumer and provides locked attach and bounded release operations
  - draft PR 199 and active task OTERYN-20260726-wiki-editorial-media-integration are the authoritative bounded owner for Wiki-to-EditorialMedia integration
  - duplicate draft PR 200 was closed without runtime changes after the ownership conflict was detected
  - PR 199 was synchronized with trusted main through merge commit aa55f44bc0e2ebc594bd966ced40b6e6d005dff3
  - PR 202 changes only this programme task record to correct the authoritative owner
  - no other open implementation PR owns the declared Wiki-to-EditorialMedia paths
  - no write occurred outside blakinio/Oteryn-Platform
  - no production, router, DSM, Internet-exposure or external-repository action occurred
  - PR 199 delivered the complete accepted Wiki-to-EditorialMedia child and squash-merged as f66c9944fd8110014773bd7cb7b58c9f49e45af0 after every required exact-head check passed
  - ADR 0014 is accepted and the runtime preserves private storage, exact Wiki permissions, current-reference synchronization, effective-publication delivery authorization and signed administrator previews
  - live open-pull-request reconciliation after PR 199 found no owner overlapping homepage, shared navigation, SEO, sitemap, robots or Open Graph closure
  - draft PR 205 is the documentation-only archival owner for the completed PR 199 child and this programme reconciliation
  - PR 206 delivered the complete homepage, navigation, localized metadata, sitemap, robots and remaining responsive/accessibility child and squash-merged as 1d063604a66dd3154f97a6f167377d54131cc516
  - production-like browser acceptance found and resolved a static robots-file shadow before PR 206 merge
derived:
  - public Wiki and Wiki administration remain complete child slices
  - the Wiki-to-EditorialMedia child is complete and its implementation paths are released
  - the source-backed initial Wiki-content child is the next independent non-overlapping bounded child
  - staging still requires exact-final-SHA redeployment after all remaining implementation children merge
unknown:
  - approved source text for gameplay-specific initial Wiki articles
  - exact current programme-level functional and visual closure record paths
  - whether Events or CMS image consumers are approved beyond Wiki-first integration
conflicts: []
first_failure:
  marker: none
  evidence: the PR 199 browser picker failure was repaired and every required exact-head workflow passed before merge
rejected_hypotheses:
  - PR 200 is the authoritative media child: live PR state and its closure comment identify PR 199 as the selected owner
  - PR 176 integrated media consumers: its merged scope explicitly excluded consumers
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
validation:
  - command: repository, task, Issue 145 and pull-request reconciliation
    result: PASS
    evidence: trusted main 45297ec561075b62c36b7350b878b46cbd7c44fc, open draft PR 199, closed duplicate PR 200 and current overlap search
  - command: authoritative child branch synchronization
    result: PASS
    evidence: PR 199 branch includes main through merge commit aa55f44bc0e2ebc594bd966ced40b6e6d005dff3
  - command: correction pull request creation
    result: PASS
    evidence: PR 202 owns only this programme checkpoint correction
  - command: Wiki-to-EditorialMedia child lifecycle
    result: PASS
    evidence: PR 199 passed required runtime and browser validation and squash-merged as f66c9944fd8110014773bd7cb7b58c9f49e45af0
  - command: homepage-navigation-seo child lifecycle
    result: PASS
    evidence: PR 206 passed every exact-head runtime, static, image, governance and critical browser check and squash-merged as 1d063604a66dd3154f97a6f167377d54131cc516
blockers:
  - none for programme coordination
next_action: Merge the homepage-navigation-seo archival pull request, then create the bounded source-backed initial Wiki-content child task, branch and draft pull request from trusted main.
```

## Notes

Commercial scope, Canary/login-server writes, production changes and router/DSM/Internet-exposure changes remain excluded.
