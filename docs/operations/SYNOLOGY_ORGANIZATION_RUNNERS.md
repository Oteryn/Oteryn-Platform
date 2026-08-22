# Oteryn Synology organization runners

Implementation tracker: `Oteryn/Oteryn#34`
Platform provider: `Oteryn/Oteryn-Platform#1199`
Atlas provider: `Oteryn/Oteryn-Atlas#35`
Game provider: `Oteryn/Oteryn-Game#34`

Status: `LIVE_SPLIT_OBSERVED_PROVIDER_CLOSEOUT_IN_PROGRESS`

## Target control plane

The intended organization control plane is:

| group | repository access contract | runner | custom label |
| --- | --- | --- | --- |
| `platform-runners` | selected repository: `Oteryn/Oteryn-Platform` only | `oteryn-synology-platform` | `oteryn-platform` |
| `atlas-runners` | selected repository: `Oteryn/Oteryn-Atlas` only | `oteryn-synology-atlas` | `oteryn-atlas` |
| `game-runners` | selected repository: `Oteryn/Oteryn-Game` only | `oteryn-synology-game` | `oteryn-game` |

All three runners register at organization URL `https://github.com/Oteryn`. Registration uses an explicit `--runnergroup`, one product label, and `--no-default-labels`.

Labels are routing metadata. Selected-repository runner-group policy is the authorization boundary.

## Current observed live state

The split registrations have been observed live on Synology with Actions Runner `2.336.0`, separate registration/work identities, and canonical immutable image:

`ghcr.io/oteryn/oteryn-deploy-runner@sha256:f0c452798a17df09006a12d437e83a72d681dcd338ef22ed01fca329d1bbab8d`

Observed registrations:

- `platform-runners` / `oteryn-synology-platform` / `oteryn-platform`;
- `atlas-runners` / `oteryn-synology-atlas` / `oteryn-atlas`;
- `game-runners` / `oteryn-synology-game` / `oteryn-game`.

Atlas replacement routing is independently proven by `Oteryn/Oteryn-Atlas` trusted-main run `32526864123`, job `96911114022`: product publication, retained FullWorld roots, HTTP Range, desktop Chromium and mobile Chromium all passed on `oteryn-synology-atlas`. Platform removed the superseded cross-repository Atlas scaffold in PR #1212.

The currently connected repository GitHub API surface cannot directly read the organization runner-group `Selected repositories` setting. Do not represent the settings-page value as read back unless a future authorized control-plane surface actually returns it. Effective cross-repository scheduling can be tested separately with bounded positive/negative routing canaries.

## Public repository fence

The product repositories are public. Organization runner-group access to each selected public repository must be intentional. No self-hosted job may run arbitrary fork or untrusted pull-request code. Provider workflows must keep privileged local jobs behind trusted `main`, `workflow_dispatch`, protected environments, or an equivalently fail-closed internal path.

## Image provenance

The deploy-runner image is built from the official Actions Runner release tarball rather than mutable `ghcr.io/actions/actions-runner:latest`.

Pinned source inputs:

- Actions Runner `2.336.0`;
- Linux x64 release SHA-256 `04cf0be1aff4c3ec3554466c39124ca250e3effd8873bb7e8d68535aa9505d5d`;
- pinned Ubuntu 24.04 amd64 base manifest;
- pinned Docker CLI 29.6.2 amd64 image manifest;
- build fails if the package lacks `externals/node24/bin/node` or reports a different runner version.

The release tarball path is deliberate: the runtime contract is checked at build time and avoids mutable runner-base identity.

## Repository bootstrap contract

`deploy/synology/runner/entrypoint.sh` supports both:

- legacy/current repository scope with exact owner/repository URL and no runner group;
- organization scope with exact organization URL plus mandatory group, runner name and strict custom label list.

Both modes use `--no-default-labels`. Organization mode also uses `--runnergroup`. The entrypoint rejects attempts to recreate GitHub default labels (`self-hosted`, OS labels and architecture labels) and reuses persistent `.runner` state without requiring registration input on ordinary restart.

`deploy/synology/runner/test-entrypoint.sh` deterministically exercises the positive organization path, legacy repository compatibility, token-file input, malformed identity/routing failures, default-label rejection and restart-safe existing registration.

## Prepared Compose target

Use:

- `deploy/synology/runner/organization.env.example` as the no-secret environment template containing only token **file paths**;
- `deploy/synology/runner/compose.organization.example.yml` as the three-runner reference definition.

Each runner has a distinct persistent config volume and work volume. Never reuse one runner's registration volume for another product.

Platform and Atlas retain raw Docker socket access only because proven host-local workloads require Docker control. Platform also retains the Platform staging-state mount. Atlas must not receive that staging-state mount. Game starts as UID/GID `1001:1001` without raw Docker socket or root execution; its provider task must prove a host-Docker requirement before either privilege is added.

A raw Docker socket is host-equivalent privilege. Separate containers and runner groups improve scheduling/repository authority isolation but are not a hard Docker-host security boundary for runners sharing the same daemon.

## First-registration / recovery procedure

For a fresh or replacement organization registration:

1. confirm the expected runner group already exists with the exact provider repository access policy;
2. use an organization-level one-time registration token in a local file outside Git;
3. start only the intended runner service;
4. verify exact runner name, group and custom label;
5. truncate the host token file after registration;
6. force-recreate only that runner with the same persistent config/work volumes;
7. verify restart from `.runner` without registration input;
8. preserve old registration state until the replacement route is proven.

Never place a token value in Git, Issues, PRs, task records, workflow logs or Compose environment values.

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

Do not route by custom label alone and do not add generic `self-hosted` eligibility.

## Provider rollout gates

### Platform

Prove exact runner name/group/label, expected Docker access, Platform staging-state access and an existing trusted Platform Synology operation before retiring the repository-scoped rollback runner.

### Atlas

Prove exact preview state, LAN endpoint, cutover/rollback and real Chromium E2E through `atlas-runners + oteryn-atlas`, then remove equivalent Atlas execution from Platform. This functional route is now proven; provider Issue #35 owns the remaining security/control-plane closeout.

### Game

Identify and prove the smallest real Game-owned local runtime/integration path through `game-runners + oteryn-game`. Ordinary deterministic Game builds/exports remain GitHub-hosted. Do not add Docker/root privilege without provider evidence.

## Rollback and retirement

Keep `oteryn-synology-staging` as rollback until all three replacement provider routes are terminally proven and no retained workflow targets it. Preserve its registration/config state and persistent product data while that condition remains false.

Retirement requires all of:

- Platform replacement PASS;
- Atlas replacement and owner-workflow transfer PASS;
- Game replacement and owner-workflow transfer PASS;
- no retained workflow targets `oteryn-staging`;
- rollback evidence recorded;
- provider Issues and `Oteryn/Oteryn#34` agree that the old runner has no remaining workload owner.

Do not delete the legacy runner merely because the new split containers exist.