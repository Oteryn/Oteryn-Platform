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
- [x] Approved initial Wiki publication content is present without invented gameplay facts.
- [x] All implementation-child required exact-head validation passes.
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
  - merged PRs #146, #157, #158, #159, #160, #161, #175, #176, #191, #192, #194, #195, #196, #197, #198, #199, #201, #203, #204, #205, #206, #207, #208 and #209
blockers:
  - none for programme coordination
cross_repository_tasks:
  - none
```

The programme task owns coordination only. Deployment and closure paths are delegated to `OTERYN-20260726-public-web-final-staging-closure`.

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
| Safe Markdown rendering boundary | COMPLETE | PRs #194 and #199; restricted CommonMark plus approved local media syntax | Preserve remote-image and raw-HTML rejection. | none |
| Wiki administration | COMPLETE | PR #196 / `f512f1e3a9bd567d40ddb09b699291c99a1b65f8` | Preserve auth + confirmed MFA + exact permission and optimistic locking. | none |
| Wiki search | COMPLETE | PR #194; published-only locale-isolated bounded search | Preserve no-draft-leak and rate-limit behavior. | none |
| Safe editorial image library foundation | COMPLETE | PR #176; reusable private media boundary | Preserve; do not duplicate storage or upload processing. | none |
| Approved media consumer integration | COMPLETE | PR #199 / `f66c9944fd8110014773bd7cb7b58c9f49e45af0`; ADR 0014; transactional references; verified published-only delivery; signed administrator previews | Preserve the accepted Wiki-only boundary; Events and CMS remain excluded. | none |
| Canonical and hreflang metadata | COMPLETE | PR #206; shared escaped metadata with freshness-aware equivalent alternates | Preserve. | none |
| Open Graph, sitemap and robots exclusions | COMPLETE | PR #206; bounded OG metadata, fail-closed published-only sitemap and authoritative dynamic robots policy | Preserve. | none |
| Responsive, keyboard and accessibility closure | COMPLETE | PRs #194, #196, #199, #206 and #208 required browser profiles | Preserve required profiles. | none |
| Initial approved Wiki content set | COMPLETE | PR #208 / `f8002191f0e5270dc4191227fd01d5e709ee5ab6`; thirteen bilingual source-backed topics and conflict-safe operator provisioning; task archived through PR #209 | Install or verify the exact reviewed package on final staging. | final-staging child |
| Final Synology staging deployment | IMPLEMENTING | Task `OTERYN-20260726-public-web-final-staging-closure`; guarded exact-SHA dispatcher, existing reviewed deploy workflow and live Chromium evidence design | Validate, merge and collect exact deployment/smoke evidence. | final-staging child |
| Issue #145 closure | PENDING | All implementation requirements and child archival lifecycles are merged; staging evidence remains outstanding | Close only after exact-SHA staging acceptance and temporary-trigger cleanup. | programme closure task |

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T17:10:00Z
head: 994f17f64b5d608094933cc35af240e954a76fc2
branch: ops/OTERYN-20260726-public-web-final-staging-closure
pr: none
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
  - trusted main is a262996eda36fc9430fe1883ea637ffd2f6ff698 after PR 209 archival merge
  - Issue 145 remains open
  - every approved implementation requirement is merged and every prior implementation child is archived
  - PR 208 delivered exactly thirteen bilingual source-backed Wiki launch topics and PR 209 archived its lifecycle
  - active task OTERYN-20260726-public-web-final-staging-closure is the only owner for final staging deployment and smoke paths
  - the final-staging branch adds a guarded main-only one-shot dispatcher, inert exact-image trigger and self-hosted live Chromium smoke while reusing the existing deploy workflow
  - no other open implementation owner overlaps the declared final-staging paths
  - no write occurred outside blakinio/Oteryn-Platform
  - no production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - programme completion now depends only on successful final-staging evidence, temporary-trigger cleanup, child archival and Issue 145 closure
unknown:
  - whether staging currently contains exactly one eligible Wiki publisher Identity
  - exact deployment and smoke workflow/run identifiers until the final-staging trigger merges
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - final staging may deploy an earlier runtime SHA: the one-shot waits for exact merge-SHA Platform and Gateway tags
  - publisher identity must be hard-coded or logged: the self-hosted job resolves exactly one eligible identity internally and fails closed otherwise
  - CI image builds prove live staging: exact container-tag checks and Chromium route assertions remain required
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
validation:
  - command: implementation and archival reconciliation
    result: PASS
    evidence: PRs 208 and 209 merged as f8002191f0e5270dc4191227fd01d5e709ee5ab6 and a262996eda36fc9430fe1883ea637ffd2f6ff698
  - command: final-staging ownership reconciliation
    result: PASS
    evidence: one bounded active child owns the temporary workflow, inert trigger, task and coordination records
blockers:
  - none before pull-request validation
next_action: Open a draft pull request for the final-staging closure and run exact-head validation.
```

## Notes

Commercial scope, Canary/login-server repository writes, production changes and router/DSM/Internet-exposure changes remain excluded.
