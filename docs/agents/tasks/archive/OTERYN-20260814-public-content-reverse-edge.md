---
task_id: OTERYN-20260814-public-content-reverse-edge
status: completed
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
---

# OTERYN-20260814 public content reverse edge

## Goal

Complete Issue #1060 as the `OTERYN_PORTAL_COMPLETION` entry-7 prerequisite: remove the pre-ADR-0033 Announcements/Events dependency on `App\PublicPortal\PublicContentState` with source-owned availability states while preserving observable homepage/source-component behavior. Federated search itself remains out of scope.

## Acceptance criteria

- [x] Announcements and Events application/view code no longer depends on `App\PublicPortal\`.
- [x] Source-owned typed states preserve `AVAILABLE`, `EMPTY`, and `UNAVAILABLE` values and existing render branches.
- [x] Existing component escaping and public behavior are preserved.
- [x] Architecture regression guard rejects the whole `App\PublicPortal` namespace root, including root aliases.
- [x] Historical material review finding was repaired and its review thread is resolved/outdated.
- [x] Candidate was reconstructed directly on protected `main` without replaying the stale 20-commit history.
- [x] All exact-final-head repository workflows passed.
- [x] Exact-head self-review passed with zero open material findings.
- [x] Runtime/browser E2E is `NOT_APPLICABLE`: routes, state bytes, rendered branches, persistence and runtime integrations were not changed; repository acceptance workflows still passed.
- [x] Issue #1060 is closed completed after merge.
- [x] Source branch was automatically deleted and its absence was verified.

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

## Terminal checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T09:19:00+02:00
status: completed
branch: refactor/issue-1060-public-content-reverse-edge
pr: 1061
final_delivery_head: 09f97728c9eaf4001a0e9898281f0d81eb3eb8bd
merge_commit: 04f8dd572785003b143eccc401466e59cc1cbf87
issue: 1060
issue_state: closed_completed
proven:
  - PR #1061 merged by squash into protected main as 04f8dd572785003b143eccc401466e59cc1cbf87.
  - Exact final delivery head 09f97728c9eaf4001a0e9898281f0d81eb3eb8bd passed CI 31930890612, Agent Governance 31930890631, Acceptance E2E and Visual UX 31930890638, Portal Acceptance Contract 31930890604, Announcements Acceptance 31930890622, Events Acceptance 31930890614, Content Scale Acceptance 31930890606, Phase 7 Production-Like Validation 31930890613, Platform DB Outage Validation 31930890637, Edge Security Emulation 31930890607, Game Auth Ticket Concurrency 31930890632 and Build Synology Staging Images 31930890629.
  - The sole material historical P2 for root namespace alias detection is resolved and outdated.
  - Issue #1060 is closed with state_reason completed.
  - Source branch ref `refactor/issue-1060-public-content-reverse-edge` is absent after merge.
  - No staging, production, protected-environment, external-repository or owner-funded AI operation was part of delivery.
derived:
  - The reverse-edge prerequisite is terminal and ownership can be released; later federated-search work may depend on the merged source-owned state boundary.
unknown: []
conflicts: []
blockers: []
validation:
  - command: exact-final-head GitHub Actions aggregate
    result: PASS
    evidence: all 12 workflows associated with 09f97728c9eaf4001a0e9898281f0d81eb3eb8bd completed successfully
  - command: PR review-thread audit
    result: PASS
    evidence: one historical P2 thread, resolved=true and outdated=true
  - command: merged outcome verification
    result: PASS
    evidence: PR #1061 merged=true, Issue #1060 closed/completed, main contains merge commit 04f8dd572785003b143eccc401466e59cc1cbf87
self_review:
  result: PASS
  exact_head: 09f97728c9eaf4001a0e9898281f0d81eb3eb8bd
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - exact-head workflow aggregate is green
    - material review finding is resolved/outdated
    - implementation merge and resulting main state verified
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository implementation task branch
source_branch_evidence: exact branch search returned no matching ref after merge
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository implementation task branch merged by squash through PR #1061
source_branch_evidence: exact branch search for refactor/issue-1060-public-content-reverse-edge returned no matching ref after merge
```
