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
- [ ] Public Wiki, administration, safe rendering, search and approved media integration are complete in English and Polish.
- [ ] Homepage, navigation, SEO, sitemap, robots, responsive and accessibility closure are complete.
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
  - merged PRs #146, #157, #158, #159, #160, #161, #175, #176, #191 and #192
blockers:
  - none for this coordination PR
cross_repository_tasks:
  - none
```

The programme task owns coordination only. Implementation paths must be claimed by bounded child tasks and draft PRs before modification.

## Implementation reconciliation

| Requirement | Status | Exact evidence | Remaining action | Proposed owning PR |
|---|---|---|---|---|
| Production homepage and shared public shell | COMPLETE | PR #146; `HomePageQuery`; `home.blade.php`; shared header/footer | Preserve and extend without duplicating the shell. | homepage-navigation-seo child |
| Dynamic world summary, published news and explicit runtime states | COMPLETE | PR #146; `HomePageQuery` and `PublicContentState` | Preserve AVAILABLE/EMPTY/STALE/UNAVAILABLE semantics. | homepage-navigation-seo child |
| Announcements and Events modules | COMPLETE | PR #157; module code, routes and tests | No domain rewrite. | homepage-navigation-seo child |
| Announcement ticker and upcoming event on homepage | MISSING | Current homepage view model contains only world and news | Compose delivered providers with truthful states and tests. | homepage-navigation-seo child |
| Download Center | PARTIAL | PR #161; routes, query, views and tests | Add shared navigation/homepage discoverability and closure acceptance. | homepage-navigation-seo child |
| Server information, beginner guide, support and legal baseline | COMPLETE | PR #159; typed editorial routes, queries and navigation | Preserve publication truth and trusted links. | homepage-navigation-seo child |
| Guild index | PARTIAL | PR #160; route, query, view and tests | Add shared navigation discoverability and closure acceptance. | homepage-navigation-seo child |
| PL/EN localization foundation | PARTIAL | PR #175; locale routing, canonical/hreflang and localized current surfaces | Extend to Wiki. | public-wiki child |
| Wiki persistence, lifecycle, revisions, locking, permissions and audit | PARTIAL | PR #158; `app/Wiki/**` and reversible foundation migration | Expose secure public/admin workflows and conflict handling. | public-wiki child |
| Public Wiki routes, categories, articles, breadcrumbs, TOC and related articles | MISSING | No Wiki route module/controller/view on trusted main | Implement published-only locale-aware public Wiki. | public-wiki child |
| Safe Markdown rendering boundary | MISSING | Input rules exist, but no maintained renderer or rendered public output exists | Add restricted renderer, anchors, TOC, internal links and security tests. | public-wiki child |
| Wiki administration | MISSING | No `/admin/wiki` routes, controllers, requests or views | Implement auth + confirmed MFA + exact permissions and bounded audit actions. | public-wiki child |
| Wiki search | MISSING | No search interface, implementation, route or tests | Implement published-only locale-isolated bounded search. | public-wiki child |
| Safe editorial image library foundation | COMPLETE | PR #176; reusable private media boundary | Reuse; do not duplicate storage. | wiki-media child |
| Approved media consumer integration | MISSING | PR #176 explicitly excluded consumers | Integrate Wiki first; other consumers require separate evidence. | wiki-media child |
| Canonical and hreflang metadata | PARTIAL | PR #175; localized SEO partial | Extend to Wiki and verify published content. | homepage-navigation-seo and public-wiki children |
| Open Graph, sitemap and robots exclusions | MISSING | No sitemap; robots allows all; layout lacks Open Graph abstraction | Add metadata, sitemap, exclusions and noindex. | homepage-navigation-seo child |
| Responsive, keyboard and accessibility closure | PARTIAL | Existing acceptance covers delivered surfaces | Add Wiki/admin/media/homepage coverage. | implementation children plus acceptance closure |
| Initial approved Wiki content set | MISSING | No Wiki publication seed exists | Add only source-backed approved content. | wiki-content child |
| Final Synology staging deployment | BLOCKED | latest proven staging SHA is `7164edd1308d9f43cfbc20fb37901e66448fe165`; implementation remains incomplete | Deploy exact final SHA through reviewed workflow and run live smoke. | deployment-closure child |
| Issue #145 closure | MISSING | Issue remains open | Update only from merged evidence and close after staging acceptance. | programme closure PR |

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T21:30:00Z
head: UNKNOWN
branch: feat/OTERYN-20260725-public-web-programme-closure
pr: 190
status: ready
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
  - branch was resynchronized from trusted main 5a5019858d0e01ba0c77e6efe88f7a23f16dbef5
  - execution mode routing exists on main through PR 191 and its task is archived through PR 192
  - Issue 145 remains open
  - no open PR owns Wiki, PublicPortal, EditorialMedia consumer, SEO or sitemap implementation paths
  - current homepage composes only world status and latest news
  - Wiki foundation has persistence, lifecycle services, revisions, exact permissions, optimistic locking and audit
  - trusted main has no public or administrator Wiki route module, Wiki search or rendered Markdown boundary
  - editorial media has no delivered consumer integration
  - canonical and hreflang support exists for current localized public routes
  - public robots.txt currently disallows nothing and no sitemap implementation was found
  - latest proven Synology staging deployment SHA is 7164edd1308d9f43cfbc20fb37901e66448fe165
  - no write occurred outside blakinio/Oteryn-Platform
  - no production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - remaining approved scope requires bounded implementation child PRs
  - runtime implementation and validation require a CODEX-capable writable checkout
  - staging requires exact-final-SHA redeployment after implementation merges
unknown:
  - approved source text for gameplay-specific initial Wiki articles
  - exact current functional/visual acceptance record paths not yet located
  - whether Events or CMS image consumers are approved beyond Wiki-first integration
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - unchecked Issue 145 boxes prove implementation is absent: merged code proves several boxes are complete
  - PR 158 delivered a usable Wiki: public/admin routes, rendering and search are absent
  - PR 176 integrated media consumers: its merged scope explicitly excluded consumers
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
validation:
  - command: repository and GitHub reconciliation
    result: PASS
    evidence: current main, Issue 145, merged/open PRs, routes and module source inspected through GitHub
  - command: required read docs/agents/EXECUTION_MODE_ROUTING.md
    result: PASS
    evidence: PR 191 merged as 6c3b9fedcd07844315c51f59b7a4f8a3abc557e1
  - command: exact-head pull-request workflows
    result: NOT_RUN
    evidence: synchronized documentation commit triggers authoritative PR checks
blockers:
  - none for merging this coordination task
next_action: Validate and merge PR 190, then create the bounded public Wiki child task, branch and draft PR from trusted main.
```

## Notes

Commercial scope, Canary/login-server writes, production changes and router/DSM/Internet-exposure changes remain excluded.
