# OTERYN-20260821 Runner Topology Evidence

Issue: `Oteryn/Oteryn-Platform#1194`
Cross-repository owner: `Oteryn/Oteryn#32`
Audited Platform base: `3f1a0eeb42a777106bef466dbcb4150d8a1bb818`
Audit date: 2026-08-21

## Corrected verdict

`SEPARATE_REPOSITORY_SCOPED_LOCAL_RUNNERS_BY_WORKLOAD_OWNER`

The earlier interim conclusion that Atlas and Game did not need local execution was incorrect. It classified only where `runs-on: oteryn-staging` is declared, rather than which repository owns the workload actually executed on Synology.

Current live source proves that the Platform self-hosted runner executes substantial Atlas-owned and Game-owned work. Therefore the absence of a self-hosted selector inside Atlas/Game is evidence of current cross-repository execution coupling, not evidence that those products have no local execution requirement.

## Proven current workload graph

`Oteryn/Oteryn-Platform/.github/workflows/repair-synology-autostart.yml` runs on `oteryn-staging` and performs all of the following on Synology:

- fetches exact `Oteryn/Oteryn-Atlas` and `Oteryn/Oteryn-Game` revisions;
- runs the Game-owned `tools/game-atlas-creatures/export.py` producer;
- builds the deterministic Atlas creature index;
- inspects and stages the persistent `oteryn-atlas-fullworld-preview` runtime;
- serves the Atlas preview on `192.168.1.2:8097`;
- performs Atlas live cutover/rollback and real Chromium desktop/mobile acceptance;
- reads/writes persistent Atlas revision roots on the Synology host.

The active Platform deployment task explicitly states that the registered Synology self-hosted runner is the narrow trusted execution path for the LAN-only Atlas preview and that the task deploys an exact Atlas revision to `192.168.1.2:8097`.

The Platform Synology Compose stack also runs the Game runtime (`canary`) alongside Platform, DB, Redis and gateway services. Thus Game has a real local runtime/integration footprint even though current orchestration is Platform-owned.

## Repository authority evidence

Atlas `AGENTS.md` states that Atlas is the derived semantic projection/read model and that Platform may coordinate Atlas contracts but is not an Atlas runtime data source. Game `AGENTS.md` states that `Oteryn/Oteryn-Game` is the canonical authority for native game server/runtime, protocol/domain code, world/content tooling and Game-owned export contracts.

The current Platform workflow therefore mixes three concerns on one privileged runner:

1. Platform staging/control-plane operations;
2. Atlas runtime/deployment/E2E operations;
3. Game-owned producer/runtime integration operations.

That coupling is the topology finding to fix.

## Live runner security evidence

Trusted-main run `32454899481`, job `96690198992`, and bounded probe run `32460223728`, job `96705516889`, prove the current runner:

- name `oteryn-synology-staging`;
- Actions Runner `2.336.0`;
- Linux/X64;
- container `oteryn-synology-staging-runner`;
- container user `0:0`;
- restart policy `always`;
- RW `/runner` and `/work` volumes;
- RW `/var/run/docker.sock` bind mount;
- RW `/var/lib/oteryn-staging-state` bind mount;
- Docker client `29.6.2`, server `24.0.2`, Compose `5.3.1`;
- live image `ghcr.io/blakinio/oteryn-deploy-runner:main` / image ID `sha256:bad8dc119e39553f5a9d958834562a44add4978e16f9a46df7c89507c06c24b8`.

Root plus RW Docker socket is effectively Docker-host privilege. That is precisely why unrelated repository workloads should not share one registration/workspace/trust boundary.

## Correct desired state

| Repository | GitHub-hosted | Local Synology execution |
| --- | --- | --- |
| `Oteryn/Oteryn` | governance/CI | none unless a future host-local need is proven |
| `Oteryn/Oteryn-Platform` | ordinary CI/test/security | **yes**: repository-scoped `oteryn-platform` for Platform staging/control-plane operations |
| `Oteryn/Oteryn-Atlas` | ordinary CI/build/contract tests | **yes**: repository-scoped `oteryn-atlas` for FullWorld preview, persistent Atlas state, live browser E2E and Atlas cutover/rollback |
| `Oteryn/Oteryn-Game` | ordinary build/export/unit/contract tests where host locality is unnecessary | **yes** for Game-owned live runtime/integration work on Synology; repository-scoped `oteryn-game` |
| archived migration backup | none | none |

Pure deterministic builds/exports should remain GitHub-hosted when they do not need local state. A repository having a local runner does not mean every job should run locally.

## Scope model

Prefer **repository-scoped registrations** for the current topology:

- `oteryn-platform` registered only to `Oteryn/Oteryn-Platform`;
- `oteryn-atlas` registered only to `Oteryn/Oteryn-Atlas`;
- `oteryn-game` registered only to `Oteryn/Oteryn-Game`;
- custom labels only; do not enable generic `self-hosted` routing;
- separate runner config/work volumes and separate persistent data mounts;
- grant Docker socket / host state only where the workload proves it is required.

Organization-level runner groups restricted one-to-one to repositories are technically possible, but currently add control-plane complexity without reducing privilege. Repository scope is the simpler least-privilege boundary.

## Migration principle

Do not break the working `oteryn-staging` path during migration.

1. Keep `oteryn-staging` operational as rollback/bootstrap.
2. Create and validate an isolated Atlas runner/container first because Atlas has directly proven LAN-local/persistent-runtime requirements.
3. Move Atlas-owned preview/deployment/E2E workflow from Platform to Atlas and prove exact live endpoint behavior.
4. Create/validate a Game runner for Game-owned live runtime/integration work; leave non-local Game CI GitHub-hosted.
5. Refactor Platform runner to Platform-only staging/control-plane responsibility.
6. Remove cross-repository Atlas/Game runtime execution from Platform only after replacement paths pass exact-head and live E2E.
7. Preserve independent rollback for each runner.

## Findings

| ID | Severity | State | Finding |
| --- | --- | --- | --- |
| `RUNNER-001` | HIGH | OPEN | Privileged live runner uses mutable pre-transfer image `ghcr.io/blakinio/oteryn-deploy-runner:main`. |
| `RUNNER-002` | MEDIUM | OPEN | Runner Dockerfile uses mutable `ghcr.io/actions/actions-runner:latest`. |
| `RUNNER-003` | INFO | PASS | Runner `2.336.0` satisfies current Node.js 24 runner prerequisite. |
| `RUNNER-004` | INFO | PASS | Current custom-label-only registration avoids generic self-hosted scheduling. |
| `RUNNER-005` | HIGH | OPEN | Atlas-owned local deployment/E2E currently runs through the privileged Platform runner instead of an Atlas-owned execution boundary. |
| `RUNNER-006` | MEDIUM | OPEN | Game-owned producer/runtime integration work is mixed into Platform's local execution boundary; separate Game-owned local validation/runtime execution where locality is required. |
| `RUNNER-007` | INFO | PASS | Permanent arbitrary pull-request code is not currently routed onto the privileged Platform runner. |

`RUNNER-001`/`RUNNER-002` remain tracked by Platform #1199. `RUNNER-005`/`RUNNER-006` require the organization runner-topology migration plan owned by META #32.

## Final architecture target

```text
Synology
|
+-- oteryn-platform runner
|   +-- repo: Oteryn/Oteryn-Platform only
|   +-- Platform staging/control plane
|
+-- oteryn-atlas runner
|   +-- repo: Oteryn/Oteryn-Atlas only
|   +-- FullWorld local state / preview / browser E2E / cutover
|
+-- oteryn-game runner
    +-- repo: Oteryn/Oteryn-Game only
    +-- Game-owned local runtime/integration validation

GitHub-hosted runners remain the default for non-local CI in all repositories.
```

This corrected verdict supersedes the earlier `KEEP_REPOSITORY_SCOPED_PLATFORM_RUNNER` conclusion.