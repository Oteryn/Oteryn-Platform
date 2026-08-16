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
  - PR #1061
  - Issue #1060
status: validating
---

# OTERYN-20260814 public content reverse edge

## Goal

Complete Issue #1060 as the `OTERYN_PORTAL_COMPLETION` entry-7 prerequisite: remove the pre-ADR-0033 Announcements/Events dependency on `App\PublicPortal\PublicContentState` with source-owned availability states while preserving observable homepage/source-component behavior. Federated search itself remains out of scope.

## Acceptance criteria

- [x] Announcements and Events application/view code no longer depends on `App\PublicPortal\`.
- [x] Source-owned typed states preserve `AVAILABLE`, `EMPTY`, and `UNAVAILABLE` values and existing render branches.
- [x] Existing component escaping and public behavior are preserved.
- [x] Architecture regression guard rejects the whole `App\PublicPortal` namespace root, including root aliases.
- [x] Historical material whole-diff review finding was repaired and its thread is resolved/outdated.
- [x] Current candidate is reconstructed directly on current protected `main` without carrying the stale 20-commit history.
- [ ] Focused tests and all exact-head required CI/governance/acceptance checks pass on the reconstructed candidate.
- [ ] Exact-head whole-diff self-review is PASS with zero unresolved material threads.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` for this behavior-preserving dependency-direction refactor; repository acceptance workflows remain mandatory.
- [ ] Issue #1060 closes only after merge; task archives in a lifecycle-only closeout PR.

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
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T08:10:00+02:00
head: 4fdefdaf518734bb1c48250ce15839be01ef4ecd
material_head: 4fdefdaf518734bb1c48250ce15839be01ef4ecd
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
  - historical #1061 material implementation head be22972e9d270b8f3702291996347396f989f808 removed the Announcements/Events reverse dependency while preserving state bytes and rendered branches
  - automated review found one P2 because the dependency guard could miss root aliases; be22972 hardened the guard to reject the entire App\\PublicPortal namespace root and the review thread is resolved
  - stale branch terminal head 9d564f116023a9e062cc29a038c741e8a1b92a7a contained the same 11 material implementation/test/view blobs plus checkpoint updates
  - comparison against current main 11c8c47b723ad669d9d23374b12149296ac3f492 found no intervening main changes to the 11 material implementation/test/view paths
  - material candidate 4fdefdaf518734bb1c48250ce15839be01ef4ecd reconstructs exactly those validated 11 blobs on current main and therefore avoids replaying 20 stale commits
  - runtime/browser E2E is semantically not applicable because routes, state values, markup branches, persistence and runtime integrations are unchanged
derived:
  - preserving the exact repaired material blob identities on a current-main parent is safer than replaying stale branch history because it retains the reviewed behavior while eliminating unrelated historical drift
unknown:
  - conclusions of fresh exact-head CI/governance/acceptance on reconstructed PR #1061
conflicts: []
first_failure:
  marker: reconstructed-checkpoint-missing-derived
  evidence: Agent Governance run 31930846300 validated every other governance stage but failed checkpoint validation because the first refreshed checkpoint omitted required field derived; this commit repairs only that metadata omission
rejected_hypotheses:
  - merge the stale 20-commit branch wholesale
  - delete PublicPortal PublicContentState globally
  - implement federated-search providers in this prerequisite slice
  - create a generic shared state dumping-ground
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
  - command: historical focused/full validation and review
    result: PASS
    evidence: historical material head be22972 passed after PHPStan and alias-guard repairs; review thread resolved
  - command: current-main path overlap comparison
    result: PASS
    evidence: no current-main changes overlap the 11 material implementation/test/view paths
  - command: reconstructed exact-head CI/governance/acceptance
    result: FAIL
    evidence: first generation Agent Governance 31930846300 failed only for missing required checkpoint field derived; other current generation workflows continue independently and this metadata-only repair triggers a fresh generation
blockers: []
next_action: require all fresh exact-head checks after this metadata repair to pass, perform exact-head whole-diff self-review, then squash merge #1061, close #1060 and archive the task.
```
