---
task_id: OTERYN-20260814-public-content-reverse-edge
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
search_first:
  - open Issues/PRs/branches matching PublicContentState, federated search, reverse dependency, or reverse edge
optional_reads:
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
---

# OTERYN-20260814-public-content-reverse-edge

## Goal

Complete Issue #1060 as the canonical `OTERYN_PORTAL_COMPLETION` entry-7 prerequisite: remove the pre-ADR-0033 `Announcements` / `Events` dependency on `App\PublicPortal\PublicContentState` with source-owned availability states, while preserving current homepage and source-component behavior. Do not implement federated search itself.

## Acceptance criteria

- [x] `Announcements` and `Events` application/view code no longer depends on `App\PublicPortal\` in the proposed diff.
- [x] Source-owned typed availability states preserve `AVAILABLE`, `EMPTY`, and `UNAVAILABLE` semantics and rendered state values.
- [x] Existing announcement/event component branches and escaping are unchanged apart from source-owned enum identity.
- [x] A regression test prevents the forbidden source -> `PublicPortal` dependency from returning, including root-namespace aliases.
- [ ] Focused tests and exact-head required CI/governance pass on a fresh merge-ref against current protected `main`.
- [x] Whole-diff self-review is terminal with no unresolved material finding on material implementation head `be22972e9d270b8f3702291996347396f989f808`.
- [x] Runtime/browser E2E is classified `NOT_APPLICABLE` for the behavior-preserving semantic diff; repository acceptance workflows remain mandatory and must pass before merge.
- [ ] Issue #1060 is closed only by a validated merged PR; task is archived in a separate closeout PR.

## Ownership

```yaml
owned_paths:
  - app/Announcements/Queries/AnnouncementTickerProvider.php
  - app/Announcements/ViewModels/AnnouncementTicker.php
  - app/Announcements/ViewModels/AnnouncementTickerState.php
  - app/Events/Queries/UpcomingEventProvider.php
  - app/Events/ViewModels/UpcomingEventSummary.php
  - app/Events/ViewModels/UpcomingEventState.php
  - resources/views/announcements/components/ticker.blade.php
  - resources/views/events/components/upcoming-summary.blade.php
  - tests/Feature/Announcements/AnnouncementsModuleTest.php
  - tests/Feature/Events/EventsModuleTest.php
  - tests/Unit/Architecture/PublicPortalSourceDependencyTest.php
  - docs/agents/tasks/active/OTERYN-20260814-public-content-reverse-edge.md
modules:
  - Announcements
  - Events
  - PublicPortal
dependencies:
  - ADR 0033 / docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - Issue #1060
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T09:18:00+02:00
head: be22972e9d270b8f3702291996347396f989f808
material_head: be22972e9d270b8f3702291996347396f989f808
branch: refactor/issue-1060-public-content-reverse-edge
pr: 1061
status: validating
context_routes:
  - OTERYN_PORTAL_COMPLETION entry 7
  - ADR 0033 federated-search reverse-edge prerequisite
owned_paths:
  - app/Announcements/Queries/AnnouncementTickerProvider.php
  - app/Announcements/ViewModels/AnnouncementTicker.php
  - app/Announcements/ViewModels/AnnouncementTickerState.php
  - app/Events/Queries/UpcomingEventProvider.php
  - app/Events/ViewModels/UpcomingEventSummary.php
  - app/Events/ViewModels/UpcomingEventState.php
  - resources/views/announcements/components/ticker.blade.php
  - resources/views/events/components/upcoming-summary.blade.php
  - tests/Feature/Announcements/AnnouncementsModuleTest.php
  - tests/Feature/Events/EventsModuleTest.php
  - tests/Unit/Architecture/PublicPortalSourceDependencyTest.php
  - docs/agents/tasks/active/OTERYN-20260814-public-content-reverse-edge.md
proven:
  - protected main f61c7b229cbf251be6e5eae0be5db55aac722242 was the exact selection base
  - live overlap search found no pre-existing owner for the exact reverse-edge cleanup before Issue #1060 was created
  - FEDERATED_SEARCH_ARCHITECTURE explicitly requires removal of the Announcements/Events PublicContentState reverse edge before provider onboarding
  - source-owned backed enums retain AVAILABLE, EMPTY, and UNAVAILABLE string values and preserve provider/view branching
  - PublicPortal PublicContentState remains intact for PublicPortal-owned composition including STALE
  - initial PHPStan failures were isolated to iterator typing in the new architecture regression test; f88dc73d922f162eefc64c0e51c4c2b2917eebe7 repaired that typing and CI passed
  - head 884cf25df4dfcadfe3b01642cb499d5a6ba7490d completed all 17 emitted workflows successfully before the final review hardening
  - automatic review identified one material P2: the dependency guard could miss `use App\PublicPortal as Portal`; material head be22972e9d270b8f3702291996347396f989f808 now rejects the entire `App\PublicPortal` namespace root and the review thread is resolved
  - whole-diff self-review 4943219841 on be22972e9d270b8f3702291996347396f989f808 is PASS
  - inherited Agent Governance failure after be22972 was caused by terminal PublicPortal Today archive evidence, not #1060 implementation code
  - Issue #1080 / PR #1081 repaired that inherited archive with mandatory source-branch closeout evidence; exact repair head 23a86cba8d005e25f9cf628c66875782d85cdcf9 passed CI 31871475079 and Agent Governance 31871475120 and merged as 860033172c8b4f1ba21d8d79263f04e2f0a49928
  - rerunning the old #1061 Governance run was insufficient because GitHub reused frozen merge-ref d24a7d9d566f50e41c51bc5e3c80c19055477279 built against pre-repair main 3c3499f38100ec15ba76f958558444c87d644c15
  - fresh merge-ref 46ef5b4eda8d85d6e8238d200f8915ef8d35a5b8 correctly merges #1061 into repaired main 860033172c8b4f1ba21d8d79263f04e2f0a49928
  - Agent Governance on aa98c8b853ef2986d5b5b7e5b608ed0d80b6431a reached the checkpoint validator and failed only because this checkpoint accidentally declared version 2 while GOVERNANCE_CONTRACT.json requires version 1; this commit repairs only that metadata value
derived:
  - runtime/browser E2E is semantically not applicable because observable state bytes, branches, routes, markup, persistence, caches, and runtime integrations are unchanged; repository-level acceptance workflows still provide regression evidence
unknown:
  - conclusions of the final fresh exact-head workflow generation after the checkpoint-version metadata correction
conflicts: []
first_failure:
  marker: phpstan-architecture-regression-test-mixed-iterator
  evidence: CI 31813995072 and Deep System Validation 31813995122 reported seven PHPStan errors in PublicPortalSourceDependencyTest; production code was not implicated and the typing defect was repaired on f88dc73
rejected_hypotheses:
  - deleting PublicPortal PublicContentState globally is unnecessary because PublicPortal still owns composition states
  - implementing federated-search providers in this slice would exceed the accepted prerequisite boundary
  - creating a generic shared state module would violate ADR 0033 no-dumping-ground guidance
  - the alias review finding did not require changing production behavior; tightening the architecture guard is sufficient
  - rerunning a historical workflow run does not refresh GitHub's frozen pull-request merge ref after main changes
changed_paths:
  - app/Announcements/Queries/AnnouncementTickerProvider.php
  - app/Announcements/ViewModels/AnnouncementTicker.php
  - app/Announcements/ViewModels/AnnouncementTickerState.php
  - app/Events/Queries/UpcomingEventProvider.php
  - app/Events/ViewModels/UpcomingEventSummary.php
  - app/Events/ViewModels/UpcomingEventState.php
  - resources/views/announcements/components/ticker.blade.php
  - resources/views/events/components/upcoming-summary.blade.php
  - tests/Feature/Announcements/AnnouncementsModuleTest.php
  - tests/Feature/Events/EventsModuleTest.php
  - tests/Unit/Architecture/PublicPortalSourceDependencyTest.php
  - docs/agents/tasks/active/OTERYN-20260814-public-content-reverse-edge.md
validation:
  - command: live overlap/ownership search
    result: PASS
    evidence: no pre-existing exact owner existed before #1060 claim
  - command: state ownership and behavior inspection
    result: PASS
    evidence: backed state bytes and provider/view branches are preserved while Announcements/Events no longer import PublicPortal state
  - command: architecture alias guard review repair
    result: PASS
    evidence: be22972 rejects `App\PublicPortal` root references including root aliases; material review thread resolved
  - command: material whole-diff self-review
    result: PASS
    evidence: review 4943219841 on be22972e9d270b8f3702291996347396f989f808
  - command: inherited PublicPortal Today closeout repair
    result: PASS
    evidence: PR #1081 head 23a86cba8d005e25f9cf628c66875782d85cdcf9 passed CI 31871475079 and Agent Governance 31871475120; merged as 860033172c8b4f1ba21d8d79263f04e2f0a49928
  - command: runtime/browser E2E semantic applicability
    result: NOT_APPLICABLE
    evidence: no observable route/API/DOM/state-value/persistence/runtime contract change in the implementation refactor
  - command: final fresh exact-head CI/governance and acceptance generation after blocker repair
    result: NOT_RUN
    evidence: this checkpoint-version correction triggers the final fresh merge-ref generation
blockers:
  - none
next_action: require the newly emitted exact-head workflows on the fresh merge-ref to pass, repeat whole-diff self-review if the material diff changes, then merge PR #1061 and archive this task
```

## Notes

Issue #1060 is the exact bounded package. No external/server repository, production/staging/protected environment, credentials, Cloudflare, signing, payment, or owner-funded Codex/OpenAI/API operation is authorized by this task.
