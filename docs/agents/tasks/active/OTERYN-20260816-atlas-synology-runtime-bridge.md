---
task_id: OTERYN-20260816-atlas-synology-runtime-bridge
status: implementing
owner: chat-github
branch: ops/atlas-synology-runtime-bridge-20260816
base_branch: main
created: "2026-08-16T10:10:00+02:00"
updated: "2026-08-16T10:10:00+02:00"
project_lane: oteryn-platform-core
execution_mode: github_only
owned_paths:
  - .github/workflows/atlas-synology-runtime-bridge-one-shot.yml
  - docs/agents/tasks/active/OTERYN-20260816-atlas-synology-runtime-bridge.md
modules:
  - github-actions
  - synology-staging-runner
dependencies:
  - existing oteryn-staging self-hosted runner
  - blakinio/Otheryn main Atlas deployment contract
blockers: []
cross_repository_tasks:
  - blakinio/Otheryn: OTH-20260815-otbm-atlas-product-readiness
---

# OTERYN-20260816 Atlas Synology runtime bridge

## Goal

Use the existing `oteryn-staging` Synology self-hosted runner as a bounded operational bridge to inspect the same NAS Docker host for the private OTBM Atlas deployment, without modifying canonical `oteryn-staging` services, exposing secrets, or using SSH.

## Authorization and boundaries

The owner explicitly requested autonomous completion of the OTBM Atlas Synology deployment. This bridge may perform read-only host discovery and may later create only Atlas-owned resources with exact names/labels after source data and ownership are proven. It must preserve all Platform staging containers, volumes, networks, runner infrastructure and unrelated Docker resources.

Forbidden:

- blanket Docker prune or unrelated cleanup;
- printing environment variables, secrets, credentials, database/application contents or unrestricted Docker inspect payloads;
- changing `oteryn-staging` canonical containers, volumes or networks;
- public Internet exposure, Cloudflare/public DNS changes, or production Platform deployment;
- owner-funded AI/Codex/OpenAI usage.

The initial one-shot is discovery-only. Any helper container is ephemeral, deterministic, read-only against host `/`, and must be removed automatically with `--rm`.

## Acceptance

1. Prove the runner identity and Docker Engine boundary.
2. Inventory Atlas-related containers/ports without exposing secrets.
3. Inspect only non-secret metadata for candidate Atlas corpora on host storage, including manifest schema/chunk/source identity and presence of required viewer/environment files.
4. Determine whether recommended Atlas project/data paths exist and whether host port `8095` is already allocated by Docker.
5. Record enough safe evidence to decide whether an Atlas-owned deployment can proceed without touching Platform staging.
6. Leave no helper container behind.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T10:10:00+02:00
head: 4fcc6eb8a636e1b291ee96fb9b805d70633d1f64
head_scope: trusted Platform main before task branch writes
branch: ops/atlas-synology-runtime-bridge-20260816
pr: none
status: implementing
phase: discover
execution_mode: github_only
proven:
  - archived OTERYN-20260813-synology-diagnostics proves an existing self-hosted runner labelled oteryn-staging with Docker diagnostic access
  - the Otheryn repository has merged the private Atlas container contract
unknown:
  - whether oteryn-staging is online now
  - whether the generated Atlas corpus is already present on Synology
  - whether port 8095 is free on the live Docker host
  - whether an Atlas container/project already exists
conflicts: []
first_failure:
  marker: none
  evidence: none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260816-atlas-synology-runtime-bridge.md
validation: []
blockers: []
next_action: add and run a one-shot read-only workflow on oteryn-staging that uses an auto-removed read-only helper container to inspect only Atlas runtime metadata on the NAS host
```
