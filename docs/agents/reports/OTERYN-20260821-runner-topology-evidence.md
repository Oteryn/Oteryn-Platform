# OTERYN-20260821 Runner Topology Evidence

Issue: `Oteryn/Oteryn-Platform#1194`
Cross-repository owner: `Oteryn/Oteryn#32`
Audited Platform base: `3f1a0eeb42a777106bef466dbcb4150d8a1bb818`
Audit date: 2026-08-21

## Verdict

`KEEP_REPOSITORY_SCOPED_PLATFORM_RUNNER`

Do **not** migrate the current Synology runner to organization scope and do **not** create Game or Atlas Synology runners now.

Current evidence shows only Platform has a real Synology execution requirement. The existing runner is a highly privileged staging/control-plane runner: it runs as root, has read-write Docker socket access and a read-write staging-state bind mount. Sharing this runner across additional public repositories would broaden the trust boundary without a demonstrated workload benefit.

The desired state for the current organization generation is therefore:

| Repository | Ordinary CI | Self-hosted desired state |
| --- | --- | --- |
| `Oteryn/Oteryn` | GitHub-hosted | `NOT_NEEDED` |
| `Oteryn/Oteryn-Game` | GitHub-hosted | `NOT_NEEDED` |
| `Oteryn/Oteryn-Platform` | GitHub-hosted by default | keep one repository-scoped Synology runner using only custom label `oteryn-staging` for trusted staging/Synology operations |
| `Oteryn/Oteryn-Atlas` | GitHub-hosted | `NOT_NEEDED` |
| archived Platform migration backup | none | `NOT_NEEDED` |

If Game or Atlas later proves a host-local workload that GitHub-hosted execution cannot satisfy, create a separate least-privilege execution boundary for that repository. Re-evaluate organization runner groups at that time; do not reuse the privileged Platform Docker-socket runner merely for administrative symmetry.

## Live runner evidence

Trusted-main Actions run `32454899481`, job `96690198992`, directly reported:

- runner version: `2.336.0`;
- runner name: `oteryn-synology-staging`;
- runner-group display: `Default`;
- job completed successfully on the Synology runner.

A bounded temporary read-only probe was then executed through PR #1198 using the retained `Synology Production Target Preflight` pull-request workflow. Run `32460223728`, job `96705516889`, reached the live runner and reported before failing only on optional JSON decoding of the UTF-8-BOM-prefixed `.runner` metadata file:

- runner context: `oteryn-synology-staging`, Linux, X64;
- container: `oteryn-synology-staging-runner`;
- container user: `0:0`;
- state: `running`;
- restart policy: `always`;
- current image reference: `ghcr.io/blakinio/oteryn-deploy-runner:main`;
- current image ID: `sha256:bad8dc119e39553f5a9d958834562a44add4978e16f9a46df7c89507c06c24b8`;
- listener version observed inside the live container: `2.336.0`;
- Docker client: `29.6.2`;
- Docker server: `24.0.2`;
- Docker Compose: `5.3.1`;
- `/runner`: read-write Docker volume;
- `/work`: read-write Docker volume;
- `/var/run/docker.sock`: read-write bind mount;
- `/var/lib/oteryn-staging-state`: read-write bind mount;
- `/runner/.runner`: mode `0644`, owner `0:0`.

The probe did **not** print environment variables, credential files, secrets, application data or a full Docker inspect payload. Its final non-zero result is not evidence of a runner failure: all required host/container facts above were collected successfully; the failure was `json.JSONDecodeError: Unexpected UTF-8 BOM` while attempting the optional selected-field parse of `.runner`.

The temporary probe workflow and temporary retained-workflow extension were restored/removed before this report was written. They are not part of the intended merge diff.

## Scope proof

Current durable runner source reinforces the already-recorded post-transfer scope:

- `deploy/synology/runner/entrypoint.sh` accepts `RUNNER_URL` only in exact repository form `https://github.com/<owner>/<repo>`;
- registration uses `--no-default-labels` and the custom label `oteryn-staging`;
- the current entrypoint has no organization URL or `--runnergroup` registration path;
- merged Platform PR #1164 recorded the post-transfer runner as repository-scoped;
- the archived PR #1176 task records that generic `self-hosted` eligibility was deliberately removed and the live registration already matched the custom-label-only boundary.

The Actions job field `runner_group_name=Default` is therefore only a group-display value and is not used as evidence that the runner is organization-scoped.

## Workflow routing inventory

Repository search found the custom `oteryn-staging` selector only in Platform. Searches across META, Game and Atlas found no `oteryn-staging` or `self-hosted` routing.

The retained Platform workflows using `runs-on: oteryn-staging` are:

| Workflow | Permanent self-hosted trigger boundary | Trust classification |
| --- | --- | --- |
| `synology-diagnostics.yml` | `workflow_dispatch` only | trusted manual |
| `repair-synology-compose-orphans.yml` | `workflow_dispatch`, job additionally requires `main` | trusted manual/main |
| `recover-synology-staging-schema.yml` | `workflow_dispatch`, `main`, `synology-staging` environment | trusted manual/main/environment |
| `character-bazaar-staging-control.yml` | trusted `main` push or `workflow_dispatch`; staging environment | trusted main/manual/environment |
| `synology-production-target-preflight.yml` | PR path runs only hosted static validation; self-hosted live job requires `workflow_dispatch` + `main` + staging environment | PR does not reach permanent self-hosted job |
| `deploy-synology-staging.yml` | `workflow_dispatch`, `main`, staging environment | trusted manual/main/environment |
| `synology-container-hygiene.yml` | PR path runs only hosted static validation; self-hosted live job requires `workflow_dispatch` + `main` + staging environment | PR does not reach permanent self-hosted job |
| `repair-synology-autostart.yml` | trusted `main` push for owned deployment paths or `workflow_dispatch`; no pull-request trigger | trusted main/manual |

This is the critical security property for a public repository: permanent pull-request code is not routed onto the privileged Synology runner.

## Node.js 24 / Actions compatibility

`PASS` for runner-version prerequisite.

The live runner is `2.336.0`. The pending Actions dependency evidence records `2.327.1` as the minimum self-hosted runner version required by Node.js 24 based `actions/upload-artifact` generations. The live Synology runner is newer than that minimum.

This does not authorize routing Game/Atlas jobs to Synology. It only removes runner-version age as a blocker for Platform workflows that legitimately use this runner.

## Security assessment

### PASS — scheduling isolation

The custom label `oteryn-staging` plus `--no-default-labels` prevents generic `self-hosted` jobs from selecting the runner. Permanent self-hosted jobs are bounded to manual/trusted-main paths rather than arbitrary pull requests.

### HIGH — host-equivalent runner privilege

The runner executes as root and has read-write `/var/run/docker.sock`. In practical trust terms, code executing on this runner has Docker-host control. It also has read-write access to the staging-state mount. This is intentional for deployment operations but requires strict repository/workflow isolation.

Consequences:

- do not share this runner with Game, Atlas or META merely to save runner installations;
- do not add generic/default labels;
- do not route untrusted `pull_request` code to it;
- keep GitHub-hosted runners as the default for build/test work that does not require Synology.

### HIGH — stale/mutable runner image provenance

The live runner currently reports:

`ghcr.io/blakinio/oteryn-deploy-runner:main`

This is a pre-transfer personal namespace and a mutable tag, while the canonical repository now lives at `Oteryn/Oteryn-Platform`. Because this image operates a root/Docker-socket runner, provenance should be tightened.

Required remediation is separate from topology migration:

1. prove the Oteryn-owned runner image is published/readable after the repository transfer;
2. pin the runtime to an immutable digest under the canonical `ghcr.io/oteryn/...` namespace;
3. preserve the current running digest as rollback evidence until the replacement runner container is healthy and scheduled;
4. never replace the working registration/config volume merely to change the image coordinate.

### MEDIUM — mutable runner build base

`deploy/synology/runner/Dockerfile` currently uses `ghcr.io/actions/actions-runner:latest`. A privileged runner image should not depend on an unreviewed moving base tag. Pinning and an intentional update path should be included in runner/container supply-chain hardening.

### UNKNOWN — credential-file mode

The audit deliberately did not read or print `.credentials`. The exact permission mode of the live credential file was not captured before the bounded probe stopped at the BOM decode step. No credential value was exposed. This unknown does not change the topology verdict, but a future runner-hardening task may validate metadata/permission bits only.

## Organization runner groups

GitHub supports organization self-hosted runner groups with repository restrictions. GitHub also warns against self-hosted runners for public repositories because forked pull requests can execute dangerous code when workflows permit it.

For the current Oteryn state, an organization-level runner group provides no demonstrated benefit because only Platform needs Synology. Moving the same root/Docker-socket runner to organization scope would add control-plane complexity and a potential future exposure path while preserving the same physical machine.

Therefore:

`ORGANIZATION_RUNNER_GROUP_MIGRATION = NOT_NEEDED_NOW`

If a future multi-repository self-hosted estate is justified, use separate groups restricted to selected repositories and separate physical/container trust boundaries where privilege differs. Do not use one shared privileged staging runner across public repositories.

## Control-plane capability

The GitHub API supports organization registration tokens and runner-group administration, but organization runner registration requires organization `Self-hosted runners: write` permission. The currently connected GitHub tool surface exposes repository/Actions operations but no organization runner/group/registration-token mutation action.

This is **not a blocker** to the current desired state because the audit verdict is to keep the working repository-scoped runner. It would become a blocker only if a later approved task actually required organization-scoped runner creation/migration.

## Findings and actions

| ID | Severity | State | Finding / action |
| --- | --- | --- | --- |
| RUNNER-001 | HIGH | OPEN | Live privileged runner image still uses mutable pre-transfer `ghcr.io/blakinio/...:main`; migrate image provenance to canonical Oteryn namespace + immutable digest without replacing working registration state. |
| RUNNER-002 | MEDIUM | OPEN | Runner Dockerfile base uses `ghcr.io/actions/actions-runner:latest`; pin/update deliberately. |
| RUNNER-003 | INFO | PASS | Runner `2.336.0` satisfies Node.js 24 minimum prerequisite. |
| RUNNER-004 | INFO | PASS | `--no-default-labels` + `oteryn-staging` isolates generic self-hosted scheduling. |
| RUNNER-005 | INFO | PASS | No retained self-hosted routing found in META, Game or Atlas. |
| RUNNER-006 | INFO | PASS | Permanent Platform pull-request paths do not execute on the Synology runner; PR-only Synology checks use GitHub-hosted jobs, with live jobs gated to manual/main paths. |

## Final desired state

```text
Oteryn organization
|
+-- Oteryn/Oteryn
|   +-- GitHub-hosted only
|
+-- Oteryn/Oteryn-Game
|   +-- GitHub-hosted only
|
+-- Oteryn/Oteryn-Platform
|   +-- GitHub-hosted: ordinary CI/test/security jobs
|   +-- repository-scoped self-hosted: oteryn-synology-staging
|       +-- label: oteryn-staging only
|       +-- use: trusted Synology/staging operations only
|       +-- Docker socket: privileged boundary; do not share
|
+-- Oteryn/Oteryn-Atlas
|   +-- GitHub-hosted only
|
+-- archived migration backup
    +-- no runner
```

This desired state should be treated as the runner verdict for `OTERYN-ORG-AUDIT-v3.10` until a concrete new workload proves that another self-hosted execution boundary is required.
