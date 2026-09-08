---
task_id: OTERYN-20260908-owner-visible-portal-polish
governing_issue: 1344
required_reads:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
search_first:
  - current live Issue #1344, protected main, archived Portal ownership and staging deploy evidence
optional_reads: []
---

# OTERYN-20260908-owner-visible-portal-polish

## Goal

Governing GitHub Issue: #1344 — make the deployed Portal visual change unmistakably owner-visible and make first-party presentation assets cache-safe after protected-main releases.

## Acceptance criteria

- [x] First-party shared Portal CSS/JS and critical home artwork use deterministic versioned URLs that change with asset content.
- [x] Home/public presentation has a materially visible composition delta on desktop and mobile while preserving truthful content and existing routes.
- [x] Existing EN/PL, responsive, keyboard/focus, reduced-motion, security and Portal contracts remain green.
- [x] Exact-head browser acceptance, CI/platform-gate and Portal Acceptance Contract pass.
- [x] Protected integration used Merge Queue only.
- [x] Automatic Synology staging deploy completed successfully and deployed public staging health was proven on the resulting release.
- [x] Owner-visible acceptance was not declared complete before deployed evidence existed.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260908-owner-visible-portal-polish.md
  - resources/views/game/layout.blade.php
  - resources/views/home.blade.php
  - resources/views/identity/layout.blade.php
  - resources/views/errors/layout.blade.php
  - resources/views/admin/layout.blade.php
  - public/css/portal-owner-visible.css
  - tests/Feature/PortalVisualPolishTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
modules:
  - web-cms
dependencies:
  - PR #1334 merged and staging deploy 34212996824 proven healthy
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T12:26:00Z
head: e242f0cd212abb3cad517d6dcbfde566e604f91d
branch: fix/1344-owner-visible-portal-polish
pr: 1348
status: completed
phase: terminal_closeout
terminal_pr_policy: archive_pending
context_routes:
  - web-cms
  - portal-completion
owned_paths:
  - resources/views/game/layout.blade.php
  - resources/views/home.blade.php
  - resources/views/identity/layout.blade.php
  - resources/views/errors/layout.blade.php
  - resources/views/admin/layout.blade.php
  - public/css/portal-owner-visible.css
  - tests/Feature/PortalVisualPolishTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
proven:
  - The prior PR #1334 integrated and deployed healthy, but owner-visible acceptance remained unsatisfied because the Portal still appeared effectively unchanged; Issue #1344 was the bounded follow-up rather than a rewrite of #1334 history.
  - The implementation added deterministic 12-hex SHA-256 content versions for shared Portal CSS/JS, home-production CSS, critical home artwork and the final owner-visible presentation layer.
  - Shared cache-safe presentation URLs cover public, identity/account, error and administration layouts; stale queryless shared-asset contracts were repaired rather than bypassed.
  - Desktop home uses a materially distinct two-column realm stage with framed native-size citadel artwork and stronger hierarchy; discovery remains 3/2/1 columns across desktop/tablet/phone.
  - Phone/tablet hierarchy is copy -> live realm status -> artwork so the authoritative world status remains visible in the initial viewport; this repaired a real responsive defect found by hosted browser acceptance.
  - Final implementation exact head ffd97e261b7d4183e2b8506b298c8b9a8673325e passed CI/platform-gate run 34220894294, Acceptance E2E and Visual UX run 34220894292, Portal Acceptance Contract 34220894358, Agent Governance 34220894332, Edge Security 34220894278, CodeQL 34220894304, DB Outage 34220894318, Game Auth 34220894313, Community Data 34220894320, Error State 34220894348, Support Moderation 34220894299, Content Scale 34220894324, Phase 7 34220894335 and staging-image build 34220894241.
  - Acceptance E2E 34220894292 passed primary Chromium smoke, browser portability, responsive, public dependency resilience and keyboard accessibility; full baseline, soak and exploratory visual profiles were intentionally skipped by that bounded workflow and are not claimed as executed.
  - PR #1348 entered the real GitHub Merge Queue at 2026-09-08T12:05:37Z; merge-group CI run 34224187611 completed success on candidate e242f0cd212abb3cad517d6dcbfde566e604f91d before protected integration.
  - PR #1348 merged through Merge Queue as protected main e242f0cd212abb3cad517d6dcbfde566e604f91d at 2026-09-08T12:08:36Z; no direct merge, force-push or protection bypass was used.
  - Resulting-main Build Synology Staging Images run 34224431667 completed success and produced the immutable Platform image ghcr.io/oteryn/oteryn-platform@sha256:44645935a0751c7bc49ee25853728fa778c1b69d034f970299587a281249f3e9 with OCI revision e242f0cd212abb3cad517d6dcbfde566e604f91d.
  - Resulting-main Deploy Synology Staging run 34224564123 job 102055431566 completed success for exact release e242f0cd212abb3cad517d6dcbfde566e604f91d; Platform was new at that exact source SHA, Gateway provenance was safely reused from 43e90f9f30da20ec6d3a4475bd3d7b9b74c5af4b, and Canary remained immutable at sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f.
  - The resulting-main deployment used the fail-closed full profile because accumulated runtime impact included the Portal acceptance test path; migrations reported nothing pending and all required runtime components were reconciled.
  - The same deployment verified Platform/Gateway/Canary bindings, internal TLS Gateway-to-Platform and Gateway-to-Canary dependencies, Gateway identity/isolation, MFA QR, public HTTPS login action through the canonical host boundary, requestless login/password-reset/signed-route canonical origins, cache-control, LAN game endpoint and World Registry route 1 online with login enabled.
  - Deploy run 34224564123 ended with `Oteryn Synology staging deployment is healthy.` and the rollback step was skipped.
  - The implementation source branch fix/1344-owner-visible-portal-polish was auto-deleted after Merge Queue integration; branch search returned no retained source ref and deploy checkout observed the deleted remote ref.
derived:
  - The previous stale-cache ambiguity is removed because the exact deployed Platform image emits content-versioned first-party Portal presentation URLs validated on the same implementation candidate by PHP and real-browser acceptance.
  - Deployed staging evidence is stronger than repository merge state alone: the guarded resulting-main deployment reached the canonical public HTTPS boundary and passed the production-like health contract on the exact merged Platform image.
  - No independent external browser fetch is claimed from the assistant environment because that fetcher cannot route the staging hostname; deployed public HTTPS probes plus exact-image provenance and exact-head browser acceptance are the durable verification chain.
unknown: []
conflicts: []
first_failure:
  marker: none-terminal
  evidence: all task-owned failures found during implementation were repaired before protected integration and resulting-main deployment
rejected_hypotheses:
  - Merge alone is sufficient completion evidence; owner-visible delivery requires resulting-main deployment and public health evidence.
  - Cache-busting should be removed to satisfy old tests; the correct repair was to version shared assets and update contracts to require same-origin deterministic versions.
  - Browser smoke should be weakened when realm status fell below the mobile fold; the production responsive hierarchy was repaired instead.
changed_paths:
  - resources/views/game/layout.blade.php
  - resources/views/home.blade.php
  - resources/views/identity/layout.blade.php
  - resources/views/errors/layout.blade.php
  - resources/views/admin/layout.blade.php
  - public/css/portal-owner-visible.css
  - tests/Feature/PortalVisualPolishTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - docs/agents/tasks/archive/OTERYN-20260908-owner-visible-portal-polish.md
validation:
  - command: final PR exact-head required/current workflows on ffd97e261b7d4183e2b8506b298c8b9a8673325e
    result: PASS
    evidence: all triggered current candidate workflows listed above completed success; required CI/platform-gate was 34220894294
  - command: Acceptance E2E and Visual UX 34220894292
    result: PASS
    evidence: Chromium smoke, portability, responsive, dependency resilience and keyboard accessibility completed success on the final implementation head
  - command: Merge Queue CI 34224187611
    result: PASS
    evidence: protected merge-group candidate e242f0cd212abb3cad517d6dcbfde566e604f91d completed required CI before integration
  - command: resulting-main Build Synology Staging Images 34224431667
    result: PASS
    evidence: immutable Platform image for exact merged source e242f0cd212abb3cad517d6dcbfde566e604f91d built successfully
  - command: resulting-main Deploy Synology Staging 34224564123 / job 102055431566
    result: PASS
    evidence: exact merged Platform image deployed through the guarded full path; public HTTPS/canonical-origin/cache-control/runtime probes passed; healthy marker emitted; rollback skipped
blockers: []
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: implementation PR #1348 merged through protected Merge Queue and the dedicated source branch has no retention purpose
source_branch_evidence: branch search after merge returned no fix/1344-owner-visible-portal-polish ref; resulting-main deploy checkout also reported the remote source ref deleted
```

## Notes

This completed task changed no production deployment policy, DNS/Cloudflare configuration, live data/payment/auth semantics, server/game repository or protected-main controls. Integration and this terminal archive remain subordinate to protected main, required `platform-gate` and Merge Queue authority.