# Oteryn Synology organization runners

Implementation tracker: `Oteryn/Oteryn#34`
Platform provider: `Oteryn/Oteryn-Platform#1199`
Atlas provider: `Oteryn/Oteryn-Atlas#35`
Game provider: `Oteryn/Oteryn-Game#34`

Status: `PREPARED_NOT_ACTIVATED`

## Target control plane

Create three organization runner groups in `Oteryn`:

| group | repository access | runner | custom label |
| --- | --- | --- | --- |
| `platform-runners` | selected repository: `Oteryn/Oteryn-Platform` only | `oteryn-synology-platform` | `oteryn-platform` |
| `atlas-runners` | selected repository: `Oteryn/Oteryn-Atlas` only | `oteryn-synology-atlas` | `oteryn-atlas` |
| `game-runners` | selected repository: `Oteryn/Oteryn-Game` only | `oteryn-synology-game` | `oteryn-game` |

The runners register at organization URL `https://github.com/Oteryn`. Registration uses an explicit `--runnergroup`, one product label, and `--no-default-labels`.

Labels are routing metadata. Selected-repository runner-group policy is the authorization boundary.

## Public repository fence

All current product repositories are public. Organization runner-group access to the selected public repository must therefore be an intentional owner action. No self-hosted job may run arbitrary fork/pull-request code. Provider workflows must keep local jobs behind trusted `main`, `workflow_dispatch`, protected environment, or an equivalently fail-closed internal path.

## Image preparation

The deploy-runner image is built from the official Actions Runner release tarball rather than `ghcr.io/actions/actions-runner:latest`.

Pinned preparation inputs:

- Actions Runner `2.336.0`;
- Linux x64 release SHA-256 `04cf0be1aff4c3ec3554466c39124ca250e3effd8873bb7e8d68535aa9505d5d`;
- pinned Ubuntu 24.04 amd64 base manifest;
- pinned Docker CLI 29.6.2 amd64 image manifest;
- build fails if the runner package does not contain `externals/node24/bin/node`.

The release tarball path is intentional: the published `ghcr.io/actions/actions-runner:2.336.0` container has a reported Node 24 packaging gap while the official 2.336.0 Linux release tarball contains Node 24.

Before activation, manually publish the reviewed deploy-runner from the exact source SHA through the existing `Build Synology Staging Images` workflow, resolve `ghcr.io/oteryn/oteryn-deploy-runner@sha256:<digest>`, and put only that immutable digest in the activation env file.

## Organization-owner activation steps

The currently connected repository GitHub surface does not expose organization runner-group or registration-token mutation. An organization owner must therefore perform these control-plane steps before the prepared Compose target can register:

1. `Oteryn -> Settings -> Actions -> Runner groups`.
2. Create `platform-runners`, set repository access to `Selected repositories`, select only `Oteryn/Oteryn-Platform`, and intentionally permit that selected public repository.
3. Repeat for `atlas-runners` -> `Oteryn/Oteryn-Atlas` only.
4. Repeat for `game-runners` -> `Oteryn/Oteryn-Game` only.
5. Under `Oteryn -> Settings -> Actions -> Runners`, generate organization-level one-time registration token(s).
6. Never paste a registration token into Git, an Issue, PR, task record or workflow log.

The configuration script will fail closed if an expected runner group does not exist.

## Prepared Synology target

Use:

- `deploy/synology/runner/organization.env.example` as the no-secret environment template;
- `deploy/synology/runner/compose.organization.example.yml` as the target three-runner Compose definition.

Each runner has a distinct persistent config volume and work volume. Do not reuse the current `runner_config` volume between new runners.

Platform retains the raw Docker socket and Platform staging-state mount because those are proven current Platform control-plane requirements. Atlas retains raw Docker socket access during the first migration because current FullWorld preview cutover/rollback requires host Docker control. Game starts without raw Docker socket; `Oteryn/Oteryn-Game#34` must prove a real requirement before that privilege is added.

A raw Docker socket means host-equivalent Docker privilege. Separate runner containers/groups improve scheduling and repository authority isolation, but they do not create a hard host security boundary when multiple runners have the same raw Docker daemon. A future stronger isolation phase should replace raw Docker control with product-bounded execution adapters or separate Docker/VM trust domains where justified.

## First registration

Create a local, untracked activation env from the example and set:

- exact `RUNNER_IMAGE` digest;
- transient `PLATFORM_RUNNER_TOKEN`;
- transient `ATLAS_RUNNER_TOKEN`;
- transient `GAME_RUNNER_TOKEN`.

Start one replacement at a time. Do not start all three blindly.

Recommended order:

1. Platform replacement;
2. Atlas replacement;
3. Game replacement.

After a runner appears online in its expected group, clear its registration token from the local activation configuration. The persistent `*_runner_config` volume is then the registration state for normal restarts.

## Workflow routing contract

Platform local jobs:

```yaml
runs-on:
  group: platform-runners
  labels: oteryn-platform
```

Atlas local jobs:

```yaml
runs-on:
  group: atlas-runners
  labels: oteryn-atlas
```

Game local jobs:

```yaml
runs-on:
  group: game-runners
  labels: oteryn-game
```

Do not use only a label for the target estate and do not add generic `self-hosted` eligibility.

## Rollout gates

### Platform

Prove exact runner name/group/label, expected Docker access, Platform staging-state access and one existing trusted Platform Synology operation. Do not remove the old `oteryn-staging` registration.

### Atlas

Move a bounded Atlas-owned FullWorld local path into Atlas. Prove exact preview state, LAN endpoint, cutover/rollback and real Chromium E2E using `atlas-runners + oteryn-atlas`. Only then remove the equivalent Atlas execution from Platform.

### Game

Identify the smallest Game-owned local runtime/integration path. Prove it through `game-runners + oteryn-game`. Keep ordinary deterministic Game build/export checks GitHub-hosted. Add host Docker privilege only when the provider task proves it is unavoidable.

## Rollback

The current runner remains the rollback path until all three replacement routes pass. Preserve:

- current runner registration/config volume;
- current runner name/label;
- current live image ID recorded by #1194;
- all product persistent data;
- existing trusted workflow path until its replacement is proven.

On replacement failure, stop the new container and route the affected operation back to the still-registered current runner. Do not delete new registration state until the failure is diagnosed, and never delete persistent product data as runner cleanup.

## Retirement gate

Retire `oteryn-synology-staging` only after:

- Platform replacement PASS;
- Atlas replacement and owner-workflow transfer PASS;
- Game replacement and owner-workflow transfer PASS;
- no retained workflow targets `oteryn-staging`;
- exact rollback evidence is recorded;
- provider Issues and META #34 agree that the old runner has no remaining workload owner.
