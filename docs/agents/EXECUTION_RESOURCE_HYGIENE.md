# Execution Resource Hygiene

```yaml
execution_resource_hygiene_policy_version: 1
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

The canonical Oteryn staging Compose project is `oteryn-staging`. Its current canonical service set is defined by `deploy/synology/compose.yml`; agents must read the current file rather than rely on this document as the service source of truth.

For portal staging cleanup:

- use the exact `com.docker.compose.project=oteryn-staging` label to establish project ownership;
- preserve the current canonical service containers defined by the trusted `main` compose file;
- refuse deletion when a non-canonical candidate is still running or canonical runtime identity/health is ambiguous;
- remove only verified stopped orphan containers;
- never remove volumes, networks, images, runner infrastructure, or containers from other projects as part of the portal-container cleanup action.

The retained `.github/workflows/synology-container-hygiene.yml` workflow implements this bounded inventory/cleanup path. Its `inventory` action is read-only; its cleanup action requires explicit confirmation and remains limited to stopped verified `oteryn-staging` orphans.
