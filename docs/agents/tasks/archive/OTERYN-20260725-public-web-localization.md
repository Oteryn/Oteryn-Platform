---
task_id: OTERYN-20260725-public-web-localization
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
search_first:
  - existing localization middleware, routes, language resources and public SEO conventions
  - active tasks and open pull requests touching public routes, CMS, Events, Downloads, Wiki or shared public views
  - merged public module translation schemas and publication rules
optional_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/contracts/PUBLIC_PORTAL_EXTENSION_CONTRACT.md
---

# OTERYN-20260725-public-web-localization

## Goal

Deliver a stable Polish and English localization foundation for completed public modules, including deterministic locale-aware routing, language selection, truthful translation publication states, locale formatting and focused browser/feature acceptance.

## Acceptance criteria

- [x] Supported public locales are exactly `en` and `pl`, with an explicit deterministic default and negotiation policy.
- [x] Locale-aware public URLs are stable and canonical; legacy non-localized bookmarks follow an intentional tested compatibility policy.
- [x] The language switcher preserves equivalent public routes where possible and never fabricates missing translated content.
- [x] Missing, incomplete, draft or stale editorial translations are explicit and are not automatically published or silently replaced with another language.
- [x] Public navigation, footer, dates, numbers, 404 and unavailable states are localized.
- [x] Existing Downloads, Events, Wiki and PublicGameData domain rules remain unchanged.
- [x] Translation-focused feature and representative browser tests pass together with required CI on the exact head.

## Final checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T19:10:00Z
status: completed
branch: feat/OTERYN-20260725-public-web-localization
pr: 175
validated_head: 3e4cdef90b753a14d4145bccc284cfc7064d6d9b
merge_sha: bbcc2fe46b6527f2d4e00031a2a382f31ddc45e5
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - database
  - testing
proven:
  - supported public locales are exactly en and pl
  - legacy root remains / while canonical localized home routes are /en and /pl
  - canonical public routes, hreflang metadata and route-preserving language switching are deterministic
  - public navigation, footer, dates, numbers, 404 and unavailable states are localized
  - News, managed pages, announcements and client release notes use additive editor-controlled translation records
  - translation states are missing, incomplete, draft, published and stale
  - Polish editorial reads require a complete published fresh translation and never substitute English source content
  - translation mutation routes preserve existing exact permissions and confirmed MFA
  - Events, Downloads, Wiki and PublicGameData domain contracts are unchanged
  - authentication, account and administrator routes remain outside the locale-prefixed namespace
  - authentication redirect behavior remains legacy-compatible at /
  - schema changes are additive and reversible
  - Canary/login-server schema and session compatibility do not change
  - no secrets, production-only configuration, machine translation, automatic content duplication or commerce are involved
  - all temporary diagnostic and one-shot workflow files were removed before merge
  - PR 175 was squash-merged after all exact-head gates passed
  - trust boundary affected: public routing and editor-controlled CMS publication only
  - authentication and authorization invariant affected: no new permission; existing exact permission plus confirmed MFA is preserved
  - rollback required: reversible migration and application revert
  - secret or production-only configuration involved: none
validation:
  - command: CI
    result: PASS
    evidence: run 30170876751 on 3e4cdef90b753a14d4145bccc284cfc7064d6d9b
  - command: Agent Governance
    result: PASS
    evidence: run 30170876852 on 3e4cdef90b753a14d4145bccc284cfc7064d6d9b
  - command: Platform DB Outage Validation
    result: PASS
    evidence: run 30170876803 on 3e4cdef90b753a14d4145bccc284cfc7064d6d9b
  - command: Game Auth Ticket Concurrency
    result: PASS
    evidence: run 30170876763 on 3e4cdef90b753a14d4145bccc284cfc7064d6d9b
  - command: Phase 7 Production-Like Validation
    result: PASS
    evidence: run 30170876768 on 3e4cdef90b753a14d4145bccc284cfc7064d6d9b
  - command: Acceptance E2E and Visual UX
    result: PASS
    evidence: run 30170876770 on 3e4cdef90b753a14d4145bccc284cfc7064d6d9b
  - command: Build Synology Staging Images
    result: PASS
    evidence: run 30170876754 on 3e4cdef90b753a14d4145bccc284cfc7064d6d9b
  - command: focused localization feature tests
    result: PASS
    evidence: 9 scenarios and 96 assertions
blockers: []
next_action: none
```

## Notes

No machine translation, automatic content duplication, commerce, Canary/login-server change or cross-repository write was introduced.
