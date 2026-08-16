---
task_id: OTERYN-20260816-atlas-preview-nas-stage
status: implementing
owner: chat-github
branch: ops/OTERYN-20260816-atlas-preview-nas-stage
base_branch: main
created: "2026-08-16T10:06:00+02:00"
updated: "2026-08-16T10:06:00+02:00"
execution_mode: chat-github
owned_paths:
  - .github/workflows/atlas-preview-nas-stage.yml
  - docs/agents/tasks/active/OTERYN-20260816-atlas-preview-nas-stage.md
---

# Stage Otheryn Atlas preview project on Synology

## Objective

Use the existing repository-scoped `oteryn-staging` Synology runner to perform the non-SSH work that is safe and independently executable for the legacy Otheryn Atlas private preview:

1. inventory the NAS read-only for an already-present generated Atlas corpus;
2. copy only the merged Atlas Container Manager project files into the recommended host project directory;
3. verify exact file hashes and that canonical `oteryn-staging` containers were not changed;
4. do not start/stop/create the Atlas runtime itself and do not modify DSM Reverse Proxy, preserving the owner requirement that final lifecycle/proxy actions remain DSM-managed.

## Authorization and boundaries

Owner explicitly requested execution of the Atlas Synology staging steps in the current conversation.

This one-shot operational branch is deliberately not a product delivery branch and must not merge. It does not modify Oteryn Platform runtime code or canonical staging services.

Forbidden:

- SSH/SCP/rsync-over-SSH;
- `docker exec`;
- Docker prune or unrelated resource cleanup;
- reading or printing secrets/environment variables;
- changing canonical `oteryn-staging` containers;
- starting/stopping the Atlas preview container;
- creating/changing DSM Reverse Proxy rules;
- rebuilding the Atlas;
- PNG/WebP migration;
- deleting or overwriting an unexpected existing Atlas project file.

Persistent deliverable intentionally retained:

```text
/volume1/docker/otheryn/atlas/project/docker-compose.yml
/volume1/docker/otheryn/atlas/project/nginx.conf
/volume1/docker/otheryn/atlas/project/.env.example
/volume1/docker/otheryn/atlas/project/SOURCE.txt
```

The workflow fails closed if any existing controlled file differs from the expected source bytes.

## Source identity

Otheryn source commit: `b9f51b01352abcda4db8df54f3b575ddc7b2532b` (merged PR #415).

Expected staged file SHA-256:

```text
64497eeaef5488a849e3a420ce2c3142d4659007fefec228d6224d14b3086d90  docker-compose.yml
e4ede6aeb53e07cd721578e85edee4038939e5531e5077cf5fff8327ff616ad2  nginx.conf
```

`.env.example` is a safe local template containing only the documented recommended Atlas data/bind/port values.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T10:06:00+02:00
head: pending
branch: ops/OTERYN-20260816-atlas-preview-nas-stage
pr: pending
status: implementing
phase: synology-stage
execution_mode: chat-github
proven:
  - repository main is 4fcc6eb8a636e1b291ee96fb9b805d70633d1f64 at task start
  - deploy/synology/README.md defines the dedicated repo-scoped runner label oteryn-staging
  - the runner has Docker Engine access and is allowed for bounded Synology staging operations
  - Otheryn PR 415 merged the Atlas preview package as b9f51b01352abcda4db8df54f3b575ddc7b2532b
unknown:
  - whether /volume1/docker/otheryn/atlas/current already exists on Synology
  - whether any alternate full-map-atlas corpus is already present on the NAS
  - live DSM reverse-proxy source URL
blockers: []
next_action: run the one-shot push workflow on the dedicated Synology runner, then preserve sanitized evidence and close/delete this branch without merge
```

Branch-Disposition: delete
Branch-Disposition-Reason: one-shot Synology operational staging workflow; no repository code from this branch belongs on main after the bounded run
