---
task_id: OTERYN-20261011-bazaar-freeze
governing_issue: 1474
required_reads: []
search_first:
  - app/Marketplace/Actions/CreateCharacterAuction.php
optional_reads: []
---

# OTERYN-20261011 Bazaar listing freeze and drain report

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet B4 (BAZAAR-FREEZE-1) of the removal plan under `docs/agents/programs/`, risk rule R3.

New Character Bazaar listings are frozen behind `marketplace.listing_frozen` (env `MARKETPLACE_LISTING_FROZEN`, fail-closed outside the isolated test harnesses). In-flight auctions keep settling through the existing reconciliation. A new command, `marketplace:drain`, reconciles and then reports the non-terminal auctions per status; it exits 0 only when listings are frozen and none remain.

## Acceptance criteria

- [x] A frozen Bazaar refuses new listings before any binding, escrow, transfer or database write; the listing form redirects to the account page with a bounded message.
- [x] Exact idempotent replay of a listing that already exists is unchanged.
- [x] `marketplace:drain` reports counts per non-terminal status, exits 0 only when frozen and drained, 2 otherwise, 1 on error or invalid limit; `--report-only` runs no reconciliation.
- [x] Portal coverage surface lists the frozen state with feature evidence; strict coverage has no `partial` surface.
- [x] The legacy token ratchet passes and no baseline is raised.
- [ ] Exact-head CI passes (Marketplace settlement recovery, idempotency and MariaDB concurrency suites included).

## Ownership

```yaml
owned_paths:
  - app/Marketplace/**
  - config/marketplace.php
  - tests/Feature/Marketplace/**
  - routes/console.php
  - scripts/acceptance/coverage/surfaces/marketplace.json
  - docs/agents/tasks/active/OTERYN-20261011-bazaar-freeze.md
  - docs/agents/tasks/archive/OTERYN-20261011-bazaar-freeze.md
modules:
  - marketplace
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
updated_at: 2026-10-11T05:00:00Z
head: UNKNOWN
branch: claude/decanary-b4-bazaar-freeze-20261011
pr: pending
status: validating
context_routes:
  - marketplace
owned_paths:
  - app/Marketplace/**
  - config/marketplace.php
  - tests/Feature/Marketplace/**
  - routes/console.php
  - scripts/acceptance/coverage/surfaces/marketplace.json
  - docs/agents/tasks/active/OTERYN-20261011-bazaar-freeze.md
  - docs/agents/tasks/archive/OTERYN-20261011-bazaar-freeze.md
proven:
  - Legacy token ratchet passes locally; runtime, tests and config counts do not rise above baseline.
  - Manifest-only portal coverage validation has no errors; the marketplace surfaces are all covered.
derived:
  - The freeze is checked after the idempotent replay lookup, so a retried request for an existing listing still returns it.
unknown:
  - PHP suite result; composer dependencies could not be installed in the authoring sandbox (PHP 8.3, no registry access), so CI is the first run of the new tests and of strict coverage with the route list.
  - Follow-up BAZAAR-FREEZE-2: the Bazaar account page still shows the sell button and the create-listing view keeps its form; both sit outside the owned paths. The create route redirects while frozen, so no form is reachable. Remove the affordance with the views and lang cleanup (plan D-11).
  - Follow-up BAZAAR-DRAIN-OPS: schedule or run `marketplace:drain` in each environment and record the zero report as evidence for plan items C3 and D-5; this packet ships the tooling only and runs nothing against live data.
  - Follow-up BAZAAR-FREEZE-3: unfreezing is a C3 step; C3 removes the config key together with the adapter switch.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/Marketplace/Actions/CreateCharacterAuction.php
  - app/Marketplace/Actions/ReportCharacterAuctionDrain.php
  - app/Marketplace/Console/DrainCharacterAuctionsCommand.php
  - app/Marketplace/Http/MarketplaceListingController.php
  - config/marketplace.php
  - routes/console.php
  - tests/Feature/Marketplace/MarketplaceListingFreezeTest.php
  - scripts/acceptance/coverage/surfaces/marketplace.json
  - docs/agents/tasks/active/OTERYN-20261011-bazaar-freeze.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
  - command: node scripts/acceptance/coverage/validate-portal-coverage.mjs --manifest-only
    result: PASS
    evidence: zero errors
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: product runtime E2E
    result: NOT_RUN
    evidence: no PHP dependencies in the sandbox; exact-head CI is the first run
blockers:
  - none
next_action: Read exact-head CI; run marketplace:drain per environment before plan item C3.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
