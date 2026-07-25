# Liquid20 Synology control

This directory contains the bounded control path for the data-only Freqtrade `liquid20-v1` collector running on the existing Synology self-hosted GitHub Actions runner.

## Runtime boundary

The control workflow:

- builds the exact Freqtrade collector commit declared in `.github/workflows/liquid20-synology-control.yml`;
- publishes the immutable image to `ghcr.io/blakinio/liquid20-collector:<full-commit-sha>`;
- deploys one container named `liquid20-collector` in fixed `acceptance` mode;
- bind-mounts only `/volume1/docker/freqtrade-liquidations/data` into the collector;
- exposes no inbound ports;
- provides no exchange keys or Freqtrade trading credentials;
- does not mount the Docker socket into the collector;
- uses `restart=no`, so an interrupted 24-hour attempt remains failed evidence rather than silently continuing.

The Oteryn runner has Docker access because it is the deployment control plane. The Liquid20 collector does not. Only the bootstrap job receives `packages: write`. Bootstrap and observation receive `issues: write` solely to update the fixed non-secret status issue; they cannot use that permission to expose raw liquidation data or secrets.

Synology's Docker engine did not permit writes to the explicit `:rw` data bind while the container root was configured `--read-only`. The collector therefore runs without Docker's root-filesystem read-only flag on this host, but retains `cap-drop ALL`, `no-new-privileges`, an isolated `/tmp` tmpfs, no ports, no Docker socket and no credentials.

## Workflow operations

`Liquid20 Synology Control` supports:

- `bootstrap` — build and publish the immutable image, then deploy acceptance mode unless a collector is already running;
- `status` — inspect the container, show a bounded log tail, list the newest run directory and publish an aggregate acceptance summary when available;
- `collect` — explicitly copy the newest run directory and container diagnostics into a GitHub Actions artifact.

The hourly schedule executes `status` only. It does not create or upload artifacts. This prevents a full GitHub Actions storage quota from blocking routine observation or the deployment queue.

A push to `main` that changes the workflow or this directory performs the initial `bootstrap`. An already running collector is preserved and is never replaced by bootstrap.

## Status board

Issue `#148` is the durable non-secret status board. Every bootstrap and hourly observation replaces its body with only:

- container state and exit code;
- immutable runtime image reference;
- latest run ID;
- container start and finish timestamps;
- operation outcome;
- timestamp and link to the controlling Actions run;
- aggregate acceptance result, failed gate names and per-source event/availability counts when the final report exists.

The issue never receives exchange credentials, Oteryn secrets or raw liquidation event data. A bounded, escaped and credential-redacted log tail is shown only for a stopped container to support failure diagnosis. This provides connector-readable visibility without direct DSM or SSH access.

## Acceptance attempts

The first uninterrupted 24-hour attempt, `liquid20-20260724T170830Z-1`, completed collection and evidence hashing successfully. It observed all 20 symbols on both exchanges, but failed exactly one frozen gate: `binance-usdm.maximum_latency_over_threshold_ratio`. All other reported gates passed, including duration, availability, disconnects, parse failures, duplicates, coverage and event volume.

Because the Freqtrade source and acceptance policy are read-only in this control task, the first response is a clean unchanged retry using the same immutable image and frozen policy. This distinguishes a transient host/network latency sample from a reproducible collector implementation issue without weakening any threshold. The failed run remains immutable under the Synology evidence path.

## Evidence retention

Full immutable evidence stays under:

```text
/volume1/docker/freqtrade-liquidations/data/runs/
```

An artifact is uploaded only after an explicit `collect` operation. After a successful upload, a marker is written to the separate control directory:

```text
/volume1/docker/freqtrade-liquidations/data/github-uploaded/<run-id>
```

The marker is not written into the immutable run directory. A failed upload remains retryable without modifying accepted source evidence.

## Failure behavior

The workflow fails closed when:

- Docker is unavailable on the runner;
- the immutable collector image cannot be built or published;
- a new container exits immediately after deployment;
- no run directory exists when collection is explicitly requested;
- artifact SHA-256 verification fails;
- the trusted runner cannot update the fixed non-secret status issue.

It does not restart or replace a running collector, weaken the Freqtrade acceptance policy, or classify a failed report as successful.
