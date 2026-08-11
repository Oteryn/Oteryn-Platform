# Execution Resource Hygiene

```yaml
execution_resource_hygiene_policy_version: 2
cleanup_default: required_when_no_longer_needed
shared_host_cleanup_default: exact_ownership_only
persistent_volume_default: preserve
blanket_prune_default: forbidden
```

## Purpose

Temporary execution resources are task-owned lifecycle state, not disposable leftovers. Any agent that creates or controls containers, Compose projects, helper services, temporary deployments, networks, disposable volumes, images, runners, test databases, or equivalent scaffolding owns safe cleanup until the resource is intentionally handed off or proven durable.

This policy controls resource lifecycle hygiene. It does not expand repository, deployment, production, credential, data, payment, live-capital, protected-environment, or external-system authority.

## Before creating a temporary resource

Establish enough non-secret identity to find the resource without guessing later:

- owning task or workflow/run identity;
- deterministic container/service/project name or labels;
- environment/host boundary;
- lifecycle classification: `ephemeral`, `shared`, or `persistent`;
- cleanup trigger or retention reason.

Prefer unique task/run labels and isolated Compose project names for ephemeral work. Do not reuse a shared production/staging project name merely for convenience when an isolated task project is sufficient.

If a resource may contain durable data, secrets, user data, game state, database state, backups, credentials, or reusable caches whose ownership is unclear, classify it as persistent until proven otherwise.

## Cleanup timing

- Remove task-owned ephemeral resources immediately after their last required use, including after validation or a failed experiment when no retry still needs them.
- Do not leave a container running or stopped for a future agent merely because cleanup is inconvenient.
- Do not wait for task archival if the resource is already unnecessary.
- Before terminal `DONE`, perform one final inventory of task-owned temporary resources and verify that no unintended resource remains.
- A resource intentionally retained as an operational deliverable must have an explicit owner, purpose, and lifecycle documented outside transient chat.

## Docker and Compose safety

On a shared Docker host:

1. Identify the exact candidate by deterministic name/ID plus ownership labels or exact Compose project/service identity.
2. Inspect state immediately before removal.
3. Prove the candidate is outside the canonical/shared runtime or is a task-owned ephemeral instance.
4. Prefer non-forced removal of stopped resources. A running resource requires separate proof that stopping it is part of the authorized task.
5. Verify the resource is gone and required canonical services remain healthy after cleanup.

For a whole task-owned ephemeral Compose project, `docker compose ... down --remove-orphans` may be appropriate when the project identity is exact and the project is not shared. Do not add `-v`/`--volumes` unless every affected volume is explicitly proven task-owned and disposable.

The following are forbidden by default on shared hosts because they infer ownership from liveness/age rather than task identity:

- `docker system prune`;
- `docker container prune`;
- `docker volume prune`;
- `docker network prune`;
- broad image pruning such as `docker image prune -a`;
- force-removing unknown/running containers;
- deleting resources only because they are stopped, old, dangling, or unfamiliar.

Repository-owner authorization for a specific destructive cleanup can override this default only for the exact stated scope; it does not authorize unrelated resources.

## Persistent data and shared infrastructure

Preserve by default:

- named volumes and bind-mounted durable data;
- databases and database storage;
- shared Redis/storage state;
- shared networks;
- canonical staging/production services;
- self-hosted runner infrastructure;
- secrets/configuration stores;
- images still referenced by retained services or rollback state;
- resources belonging to another Compose project, repository, task, or application.

Before deleting any persistent/named volume, durable bind-mount content, shared network, or rollback image, require explicit scope and evidence of ownership, disposability, and required backup/rollback handling.

## Workflow responsibility

Automation that creates ephemeral resources should clean them in `always()`/finally/trap semantics where safe. Cleanup must be idempotent and exact-targeted.

A cleanup step must not hide the original failure. Preserve both the primary operation result and cleanup result when cleanup fails.

Temporary resources should carry a task/run identity so a replacement agent can safely determine whether they are stale. A worker that cannot prove ownership must preserve the resource and record it as unresolved rather than guessing.

## GitHub Actions CI specialization

GitHub Actions cleanup depends on the runner boundary. Agents must inspect the workflow's actual `runs-on`, container/service configuration, and any custom Docker commands before deciding whether cleanup is required.

### GitHub-hosted runners

For jobs running on GitHub-hosted ephemeral runners such as `ubuntu-latest`, `windows-latest`, or `macos-*`:

- the runner machine is ephemeral and is discarded after the job;
- containers declared through the workflow `services:` or job `container:` mechanism are runner-managed job resources and are torn down with the job;
- do not add `docker system prune`, `docker container prune`, or similar blanket cleanup merely to clean GitHub-hosted CI;
- do not treat GitHub-hosted service containers as Synology/self-hosted leftovers;
- explicit cleanup is still required for resources the workflow creates outside the runner lifecycle, such as external cloud resources, remote test deployments, or persistent services on another host;
- custom processes or files that can affect later steps in the same job should still use normal shell `trap`/finally cleanup when their lifetime is shorter than the job.

The presence of an automatically generated `Stop containers` phase in a GitHub Actions job is acceptable evidence that runner-managed service containers were torn down. Do not duplicate that cleanup with broad Docker commands.

### Self-hosted runners

A self-hosted runner is persistent infrastructure. Never assume the host filesystem, Docker daemon, networks, images, or custom containers are discarded after a job.

For workflows that execute on a self-hosted runner:

- built-in GitHub Actions `services:`/job containers may rely on runner-managed teardown, but agents must not assume that this covers custom `docker run`, `docker compose`, helper containers, networks, temporary directories, databases, or remote resources created by workflow steps;
- any custom task/run-owned Docker or Compose resources must use deterministic run-scoped identity and an explicit cleanup step guarded with `if: ${{ always() }}` or equivalent finally/trap semantics;
- cleanup must verify that the exact task/run-owned resources are absent afterward;
- cancellation and primary-step failure must not skip cleanup of custom resources;
- shared runner containers, runner registration/configuration, caches, canonical staging services, named volumes, and unrelated workloads must be preserved;
- blanket Docker prune is forbidden on the persistent host unless the repository owner explicitly authorizes that exact scope;
- if a self-hosted workflow intentionally leaves a resource running, the workflow/task must record the durable owner, purpose, retention reason, and later cleanup authority.

A pull request that introduces custom Docker/Compose execution on a self-hosted runner is not ready to merge while cleanup semantics or intentional retention remain undefined.

### GitHub artifacts and caches

CI storage is also lifecycle state:

- diagnostic artifacts should set an explicit `retention-days` appropriate to their purpose when the action supports it;
- do not upload secrets, environment dumps, credentials, raw production data, or unrestricted Docker inspect output as artifacts;
- caches are reusable acceleration state, not disposable containers; use deliberate cache keys and do not delete shared caches as part of container cleanup unless separately authorized;
- an agent should not create long-lived artifacts merely as a substitute for durable repository documentation or task checkpoints.

### External hosts reached from GitHub Actions

A GitHub workflow that connects to Synology or another persistent host is an external/shared-host operation, even though the workflow itself runs in GitHub Actions. Resources created there follow the shared-host and self-hosted safety rules in this document; GitHub job completion does not prove that remote resources were cleaned.

## Verification and evidence

Record only sanitized operational evidence. Useful evidence includes:

- exact non-secret resource names or IDs;
- Compose project/service labels;
- running/stopped state;
- cleanup command category and result;
- post-cleanup absence;
- health of required retained services.

Never publish environment variables, secrets, credentials, database contents, private keys, tokens, full Docker inspect output, or sensitive mounted-file contents merely to prove cleanup.

## Blocked cleanup

If required cleanup cannot be completed because access, permissions, connectivity, environment protection, ownership, or safety classification is unresolved:

- do not silently abandon the resource;
- record each exact remaining resource or the smallest safe identifying predicate;
- record why removal is unsafe/unavailable;
- record the observed state and last verification time;
- leave exactly one concrete next action;
- use `blocked` or `waiting` when cleanup is the remaining terminal requirement.

Do not return `DONE` while an unintended task-owned ephemeral resource remains and the task still owns its cleanup.

## Synology / Oteryn staging specialization

The canonical Oteryn staging Compose project is `oteryn-staging`. Its current canonical service set is defined by the trusted `deploy/synology/compose*.yml` files; agents must inspect current `main` rather than rely on this document as the service source of truth.

For portal staging cleanup:

- use the exact `com.docker.compose.project=oteryn-staging` label to establish project ownership;
- preserve the current canonical service containers defined by the trusted `main` Compose sources;
- refuse deletion when a non-canonical candidate is still running or canonical runtime identity/health is ambiguous;
- remove only verified stopped orphan containers;
- never remove volumes, networks, images, runner infrastructure, or containers from other projects as part of the portal-container cleanup action.

The retained `.github/workflows/synology-container-hygiene.yml` workflow implements this bounded inventory/cleanup path. Its `inventory` action is read-only; its cleanup action requires explicit confirmation and remains limited to stopped verified `oteryn-staging` orphans.
