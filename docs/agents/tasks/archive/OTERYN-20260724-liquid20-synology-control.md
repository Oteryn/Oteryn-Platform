---
task_id: OTERYN-20260724-liquid20-synology-control
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

## Ownership

```yaml
owned_paths:
  - .github/workflows/liquid20-synology-control.yml
  - deploy/liquid20/synology-control.sh
  - deploy/liquid20/publish-status.sh
  - deploy/liquid20/README.md
  - docs/agents/tasks/archive/OTERYN-20260724-liquid20-synology-control.md
modules:
  - Synology staging operations
  - GitHub Actions self-hosted runner control
  - Liquid20 data-only research collection
dependencies:
  - existing online self-hosted runner labeled oteryn-staging
  - read-only Freqtrade source commit c00a091c5adc67cf75c46db5805e358ffc72fad7
  - Synology host data path /volume1/docker/freqtrade-liquidations/data
  - fixed non-secret status issue 148
blockers: []
cross_repository_tasks:
  - blakinio/freqtrade remained read-only; its exact approved commit was consumed as image build input
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T22:17:35Z
head: UNKNOWN
branch: docs/OTERYN-20260727-liquid20-acceptance-complete
pr: 216
status: ready
context_routes:
  - testing
  - security
  - architecture
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260724-liquid20-synology-control.md
  - docs/agents/tasks/archive/OTERYN-20260724-liquid20-synology-control.md
proven:
  - Issue 148 reports completed run liquid20-20260725T212201Z-1 using immutable image ghcr.io/blakinio/liquid20-collector:c00a091c5adc67cf75c46db5805e358ffc72fad7.
  - The container ran uninterrupted from 2026-07-25T21:22:01Z through 2026-07-26T21:22:16Z and exited successfully with code 0.
  - The final multi-source acceptance report states passed=true with zero failed gates.
  - Both bybit-linear and binance-usdm observed all 20 frozen symbols with intersection and union counts of 20.
  - Bybit recorded 835 events with availability 0.999992 and zero disconnects per hour.
  - Binance recorded 1519 events with availability 0.999991 and zero disconnects per hour.
  - All six final evidence files passed SHA-256 verification.
  - Issue 148 states the entire immutable run directory remains on the Synology data volume and hourly monitoring is metadata-only.
  - The symbol universe, 86400-second duration, event schema, thresholds, frozen policy and collector security boundary were unchanged.
derived:
  - Liquid20 Synology acceptance is complete and the task can be archived without an infrastructure or collector change.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: final report for liquid20-20260725T212201Z-1 passed all frozen gates
rejected_hypotheses:
  - A policy relaxation is required: disproven by the unchanged retry passing every frozen gate.
  - An implementation or Synology infrastructure repair is required: disproven by successful uninterrupted execution, hashing and final evaluation.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260724-liquid20-synology-control.md
  - docs/agents/tasks/archive/OTERYN-20260724-liquid20-synology-control.md
validation:
  - command: GitHub issue 148 final status publication
    result: PASS
    evidence: passed=true, failed_gates=0, run liquid20-20260725T212201Z-1
  - command: immutable evidence hash verification
    result: PASS
    evidence: binance-usdm-summary.json, binance-usdm.ndjson, bybit-linear-summary.json, bybit-linear.ndjson, multi-source-acceptance-report.json and multi-source-manifest.json reported OK
blockers: []
next_action: Merge PR 216 after repository checks pass.
```

## Notes

The completed evidence remains immutable under `/volume1/docker/freqtrade-liquidations/data/runs/liquid20-20260725T212201Z-1`. This task did not modify Freqtrade source, trading behavior, credentials, network exposure, symbol coverage, duration, schema, thresholds or frozen acceptance policy.
