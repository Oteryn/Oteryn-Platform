# OTERYN-20260811 Tibia client analysis runtime

## Objective

Materialize the current official Linux Tibia client package on the Synology Oteryn staging host in an isolated analysis container so the executable/package layout can be inspected for map-protocol research.

## Scope

- Repository: `blakinio/Oteryn-Platform`
- Runner: `oteryn-staging`
- Environment boundary: Synology staging host
- Owned repository paths:
  - `.github/workflows/tibia-client-analysis-one-shot.yml`
  - `docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md`
- Owned runtime identity:
  - container `oteryn-tibia-client-analysis`
  - bind path `/volume1/docker/oteryn/tibia-analysis`
  - labels `com.blakinio.owner=oteryn`, `com.blakinio.purpose=tibia-client-analysis`

## Safety and lifecycle

The workflow must not modify, stop, restart, remove, or reconfigure the canonical `oteryn-staging` Compose services, the deploy runner, databases, networks, volumes, or unrelated containers. No blanket Docker cleanup is allowed. The analysis container is intentionally retained after bootstrap because its downloaded package is the immediate input to the next binary-analysis phase. Persistent package data under `/volume1/docker/oteryn/tibia-analysis` must be preserved until that phase finishes; later cleanup must target only the exact owned container/path after explicit verification that the data is no longer needed.

## Acceptance

- The job runs on exact label `oteryn-staging`.
- Host Docker access is verified before creation.
- `oteryn-tibia-client-analysis` is created or safely reused only when ownership labels match.
- The Linux launcher is downloaded from CipSoft/Tibia infrastructure and checksummed.
- The launcher runs headlessly long enough to materialize the current package, or failure evidence identifies the exact blocker.
- The job reports candidate package directories, large files, ELF identity, and non-secret package metadata.
- Canonical staging services are not intentionally modified.

## Context checkpoint

- PROVEN: canonical `deploy-synology-staging.yml` uses `runs-on: oteryn-staging` and has Docker access.
- PROVEN: Oteryn execution-resource policy requires exact ownership and forbids blanket cleanup on the shared host.
- UNKNOWN: exact current CipSoft package path and final game-client executable identity until the launcher runs on Synology.
- next_action: run the bounded one-shot bootstrap on `oteryn-staging` and inspect terminal job evidence.
