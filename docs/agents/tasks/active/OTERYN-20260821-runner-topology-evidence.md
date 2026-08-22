---
task_id: OTERYN-20260821-runner-topology-evidence
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
search_first:
  - oteryn-staging
  - self-hosted
  - runner
optional_reads: []
---

# OTERYN-20260821-runner-topology-evidence

## Goal

Provide live Platform/Synology evidence required by `Oteryn/Oteryn#32` to decide the Oteryn runner topology. Audit/validation only; no destructive runner mutation.

## Acceptance criteria

- [x] Exact live runner version/name and scheduling evidence recorded.
- [x] Docker-socket, runtime user and persistent runner/work boundaries proven without secret disclosure.
- [x] Current self-hosted routing and trigger trust boundaries classified.
- [x] Node.js 24 runner compatibility decided.
- [x] Cross-repository workload ownership inspected, not only `runs-on` placement.
- [x] Corrected organization-vs-repository desired state recorded back to META #32.
- [x] Temporary probe changes removed from terminal diff.

## Corrected verdict

`SEPARATE_REPOSITORY_SCOPED_LOCAL_RUNNERS_BY_WORKLOAD_OWNER`

The earlier interim verdict was wrong. Current Platform `oteryn-staging` does not execute only Platform work. It also executes Atlas-owned and Game-owned local workloads.

Proven examples from current `repair-synology-autostart.yml`:

- fetch exact `Oteryn/Oteryn-Atlas` and `Oteryn/Oteryn-Game` revisions;
- run Game-owned creature export tooling;
- build Atlas index;
- operate persistent `oteryn-atlas-fullworld-preview` state;
- serve `192.168.1.2:8097`;
- perform Atlas live cutover/rollback and real Chromium E2E.

Platform Synology Compose additionally hosts the `canary` Game runtime. Atlas authority says Platform may coordinate contracts but is not an Atlas runtime data source; Game authority owns native runtime and Game-owned exports.

Therefore Atlas and Game do have local execution requirements; those requirements are currently hidden behind a Platform-owned runner/workflow boundary.

## Desired state

- Platform: repo-scoped `oteryn-platform` for Platform staging/control-plane only.
- Atlas: repo-scoped `oteryn-atlas` for local FullWorld state/preview/live E2E/cutover.
- Game: repo-scoped `oteryn-game` for Game-owned local runtime/integration work; non-local builds/exports remain hosted.
- META: GitHub-hosted unless a future local requirement is proven.

Do not use one organization-wide privileged runner. Keep registrations/workspaces/mounts isolated per repository and retain custom-label-only routing.

## Live evidence

Trusted-main run `32454899481`, job `96690198992`, and bounded probe run `32460223728`, job `96705516889` prove:

- runner `oteryn-synology-staging`, Actions Runner `2.336.0`, Linux/X64;
- container `oteryn-synology-staging-runner`, user `0:0`, restart `always`;
- Docker client `29.6.2`, server `24.0.2`, Compose `5.3.1`;
- RW `/runner`, `/work`, `/var/run/docker.sock`, `/var/lib/oteryn-staging-state`;
- live image `ghcr.io/blakinio/oteryn-deploy-runner:main`, image ID `sha256:bad8dc119e39553f5a9d958834562a44add4978e16f9a46df7c89507c06c24b8`.

The optional `.runner` parse failed only on UTF-8 BOM after required bounded facts were emitted; no secrets or credential contents were printed.

## Findings

- `RUNNER-001` HIGH: mutable pre-transfer runner image; tracked by #1199.
- `RUNNER-002` MEDIUM: mutable `actions-runner:latest` base; tracked by #1199.
- `RUNNER-003` PASS: runner version satisfies current Node.js 24 requirement.
- `RUNNER-004` PASS: current `--no-default-labels` + custom label prevents generic self-hosted scheduling.
- `RUNNER-005` HIGH: Atlas local runtime/deploy/E2E is executed through the Platform privileged boundary.
- `RUNNER-006` MEDIUM: Game-owned local producer/runtime integration is mixed into Platform local execution.
- `RUNNER-007` PASS: permanent arbitrary PR code is not routed onto the privileged Synology runner.

## Migration safety

Preserve working `oteryn-staging` as bootstrap/rollback. Create and prove Atlas runner first, then Game runner, then narrow/rename Platform runner to Platform-only. Separate config/work volumes and grant host mounts/Docker access only where required. Do not remove the current path until replacement live E2E and rollback have passed.

## Checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-21T08:10:00Z
branch: audit/issue-1194-runner-topology-evidence
pr: 1198
status: validating
phase: closeout
task_kind: audit
implementation_authorized: false
proven:
  - live privileged runner identity/capability evidence captured
  - Atlas local deployment/E2E currently executes on Platform runner
  - Game producer and local Canary runtime participate in Synology execution
  - temporary probe changes are absent from final diff
derived:
  - direct self-hosted selector absence in Atlas/Game cannot be used to infer no local workload
  - steady-state least privilege is separate repository-scoped local runners by workload owner
unknown:
  - exact final least-privilege Docker/API mechanism for Atlas/Game replacement runners; implementation audit must decide whether full docker.sock is actually required
blockers:
  - Oteryn/Oteryn-Platform#1191
  - Oteryn/Oteryn-Platform#1193
next_action: keep PR 1198 Draft until unrelated Platform lifecycle cleanup lands; then refresh, validate corrected evidence, and merge normally
```

Canonical evidence report: `docs/agents/reports/OTERYN-20260821-runner-topology-evidence.md`.
Cross-repository owner: `Oteryn/Oteryn#32`.