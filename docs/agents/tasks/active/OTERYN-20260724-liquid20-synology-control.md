---
task_id: OTERYN-20260724-liquid20-synology-control
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - .github/workflows/deploy-synology-staging.yml
search_first:
  - active Synology runner and deployment workflow ownership
  - existing self-hosted runner labels and Docker control patterns
  - existing GHCR login and immutable image conventions
optional_reads: []
---

# OTERYN-20260724-liquid20-synology-control

## Goal

Use the existing `oteryn-staging` self-hosted GitHub Actions runner on Synology to build, publish, deploy, monitor and collect evidence from the data-only Freqtrade `liquid20-v1` liquidation collector without requiring manual DSM log or file transfer steps.

## Acceptance criteria

- [x] A reviewed workflow builds an immutable Liquid20 image from the exact approved Freqtrade commit and publishes it to GHCR.
- [x] The workflow deploys acceptance mode only when it will not interrupt an already running collector.
- [x] Scheduled monitoring reports container state and aggregate acceptance status without restarting the collector or automatically uploading artifacts.
- [ ] A completed run can be copied from the Synology bind mount and uploaded once after an explicit `collect` request.
- [x] The collector receives no exchange keys, trading credentials, Docker socket, inbound ports or restart policy.
- [x] A full uninterrupted 24-hour attempt completed and retained immutable evidence.
- [ ] A full uninterrupted 24-hour attempt passes every frozen acceptance gate.

## Ownership

```yaml
owned_paths:
  - .github/workflows/liquid20-synology-control.yml
  - deploy/liquid20/synology-control.sh
  - deploy/liquid20/publish-status.sh
  - deploy/liquid20/README.md
  - docs/agents/tasks/active/OTERYN-20260724-liquid20-synology-control.md
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
  - blakinio/freqtrade is read-only in this task; its exact approved commit is consumed as image build input
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T21:20:00Z
head: 7563ac0fcba737f5ecb2e280f813307d3c1b2cbc
branch: fix/OTERYN-20260725-liquid20-acceptance-retry
pr: none
status: implementing
context_routes:
  - testing
  - security
  - architecture
owned_paths:
  - .github/workflows/liquid20-synology-control.yml
  - deploy/liquid20/synology-control.sh
  - deploy/liquid20/publish-status.sh
  - deploy/liquid20/README.md
  - docs/agents/tasks/active/OTERYN-20260724-liquid20-synology-control.md
  - docs/agents/tasks/archive/OTERYN-20260724-liquid20-synology-control.md
proven:
  - PRs 147, 149, 150, 151, 152, 154 and 155 established the reviewed Synology runner control plane, connector-readable status board, bounded diagnostics, metadata-only monitoring and writable evidence volume.
  - Immutable collector image ghcr.io/blakinio/liquid20-collector:c00a091c5adc67cf75c46db5805e358ffc72fad7 ran uninterrupted for 86400 seconds as run liquid20-20260724T170830Z-1.
  - Collector execution completed with collector=0 and evidence hashes verified with hashes=0.
  - Both exchanges observed all 20 frozen symbols.
  - Bybit produced 771 events with availability 0.998885 and 0.292 disconnects/hour.
  - Binance produced 1777 events with availability 0.999991 and zero disconnects/hour.
  - The first full run failed exactly one frozen gate: binance-usdm.maximum_latency_over_threshold_ratio.
  - The failed run and its final report remain immutable under /volume1/docker/freqtrade-liquidations/data/runs/liquid20-20260724T170830Z-1.
  - Freqtrade source and the frozen acceptance policy remain unchanged and read-only in this task.
derived:
  - The single latency-ratio miss may be transient host/network behavior or a reproducible timestamp/collector issue; one unchanged retry is required to distinguish them without weakening policy.
  - A documentation update under deploy/liquid20 safely triggers bootstrap after merge while preserving the completed failed run.
unknown:
  - Exact Binance latency-over-threshold ratio and distribution are not currently exposed by issue 148.
  - Whether an unchanged second 24-hour attempt will pass the frozen Binance latency ratio gate.
  - Whether a repeated failure would require a separately authorized change in the read-only Freqtrade source.
conflicts: []
first_failure:
  marker: binance-usdm.maximum_latency_over_threshold_ratio
  evidence: issue 148 final report for run liquid20-20260724T170830Z-1
rejected_hypotheses:
  - Weaken the 5000 ms threshold or 0.01 maximum ratio: prohibited by the frozen acceptance policy.
  - Change the 20-symbol universe or 24-hour duration: prohibited.
  - Mutate or delete the failed run: prohibited; evidence remains immutable.
  - Patch the read-only Freqtrade source from Oteryn while retaining the original collector commit identity: rejected because it would break provenance.
changed_paths:
  - deploy/liquid20/README.md
  - docs/agents/tasks/active/OTERYN-20260724-liquid20-synology-control.md
validation:
  - command: first 24-hour acceptance run liquid20-20260724T170830Z-1
    result: FAIL
    evidence: one frozen Binance latency ratio gate failed; collection and hashes succeeded
  - command: retry-trigger PR checks
    result: NOT_RUN
    evidence: PR not opened yet
blockers:
  - none
next_action: Open and validate the unchanged retry PR, merge it, confirm a new run ID is running, then wait for the second full 24-hour evaluator result.
```

## Notes

This task does not modify Freqtrade strategy, execution, DCA, leverage or protected holdout behavior. It operates only on public liquidation market-data collection.
