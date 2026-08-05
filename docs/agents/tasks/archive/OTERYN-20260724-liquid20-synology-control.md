---
task_id: OTERYN-20260724-liquid20-synology-control
repository: blakinio/Oteryn-Platform
historical_pull_request: 216
historical_head: bd7c573d9bf6f3cb247e88b87ffa02aa7c412fb3
merge_commit: 49d887e843c8eae3e0ade215ca9cf44f94c4de20
completed_at: 2026-07-26T22:18:22Z
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
search_first:
  - issue 148 final Liquid20 acceptance state
optional_reads: []
---

# OTERYN-20260724-liquid20-synology-control

## Goal

Use the existing `oteryn-staging` self-hosted GitHub Actions runner on Synology to build, publish, deploy, monitor and preserve evidence from the data-only Freqtrade `liquid20-v1` liquidation collector.

## Acceptance criteria

- [x] A reviewed workflow builds an immutable Liquid20 image from the exact approved Freqtrade commit and publishes it to GHCR.
- [x] The workflow deploys acceptance mode only when it will not interrupt an already running collector.
- [x] Scheduled monitoring reports container state and aggregate acceptance status without restarting the collector or automatically uploading artifacts.
- [x] A completed run can be copied from the Synology bind mount and uploaded once after an explicit `collect` request.
- [x] The collector receives no exchange keys, trading credentials, Docker socket, inbound ports or restart policy.
- [x] A full uninterrupted 24-hour attempt completed and retained immutable evidence.
- [x] A full uninterrupted 24-hour attempt passed every frozen acceptance gate.

## Terminal ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260724-liquid20-synology-control.md
live_lease: none
live_claim: none
released_historical_paths:
  - .github/workflows/liquid20-synology-control.yml
  - deploy/liquid20/synology-control.sh
  - deploy/liquid20/publish-status.sh
  - deploy/liquid20/README.md
  - docs/agents/tasks/active/OTERYN-20260724-liquid20-synology-control.md
modules:
  - historical Synology staging operations evidence
dependencies: []
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T20:50:00Z
head: bd7c573d9bf6f3cb247e88b87ffa02aa7c412fb3
branch: docs/OTERYN-20260727-liquid20-acceptance-complete
pr: 216
status: completed
context_routes:
  - testing
  - security
  - architecture
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260724-liquid20-synology-control.md
proven:
  - Pull request 216 is closed and merged as 49d887e843c8eae3e0ade215ca9cf44f94c4de20 from final head bd7c573d9bf6f3cb247e88b87ffa02aa7c412fb3.
  - Issue 148 reports completed run liquid20-20260725T212201Z-1 using immutable image ghcr.io/blakinio/liquid20-collector:c00a091c5adc67cf75c46db5805e358ffc72fad7.
  - The container ran uninterrupted from 2026-07-25T21:22:01Z through 2026-07-26T21:22:16Z and exited successfully with code 0.
  - The final multi-source acceptance report states passed=true with zero failed gates.
  - Both bybit-linear and binance-usdm observed all 20 frozen symbols with intersection and union counts of 20.
  - Bybit recorded 835 events with availability 0.999992 and zero disconnects per hour.
  - Binance recorded 1519 events with availability 0.999991 and zero disconnects per hour.
  - All six final evidence files passed SHA-256 verification.
  - The historical source branch remains present but is classified as a retained merged branch with no live claim, lease, dependency or ownership.
  - This archive is the sole durable record for the completed historical task.
derived:
  - Liquid20 Synology acceptance is complete and no implementation, workflow, deployment or external-system ownership remains attached to this historical task.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: final report for liquid20-20260725T212201Z-1 passed all frozen gates
rejected_hypotheses:
  - A policy relaxation was required; the unchanged retry passed every frozen gate.
  - An implementation or Synology infrastructure repair was required; uninterrupted execution, hashing and final evaluation succeeded.
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260724-liquid20-synology-control.md
  - docs/agents/tasks/active/OTERYN-20260724-liquid20-synology-control.md
validation:
  - command: GitHub pull request 216 terminal-state verification
    result: PASS
    evidence: merged=true, final head bd7c573d9bf6f3cb247e88b87ffa02aa7c412fb3, merge commit 49d887e843c8eae3e0ade215ca9cf44f94c4de20
  - command: GitHub issue 148 final status publication
    result: PASS
    evidence: passed=true, failed_gates=0, run liquid20-20260725T212201Z-1
  - command: immutable evidence hash verification
    result: PASS
    evidence: six final evidence files reported OK in the preserved historical record
blockers: []
next_action: none
```

## Notes

The completed evidence remains immutable under `/volume1/docker/freqtrade-liquidations/data/runs/liquid20-20260725T212201Z-1`. This historical task did not modify Freqtrade source, trading behavior, credentials, network exposure, symbol coverage, duration, schema, thresholds or frozen acceptance policy. Later maintenance requires a new task and claim.