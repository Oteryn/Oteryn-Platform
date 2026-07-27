---
task_id: OTERYN-20260727-synology-production-target-preflight
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - deploy/synology/README.md
  - deploy/synology/compose.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/scripts/rollback.sh
  - .github/workflows/deploy-synology-staging.yml
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
search_first:
  - active tasks and open pull requests touching deploy/synology or Synology workflows
  - existing Synology runtime health rollback and evidence mechanisms
  - production-only facts that must remain UNKNOWN
optional_reads: []
---

# OTERYN-20260727-synology-production-target-preflight

## Goal

Add and execute a deterministic production-target preflight against the existing local Synology staging stack. Prove the host/runtime, storage, binding, health, least-privilege, persistence, restore-drill and rollback-readiness boundaries that can be verified locally, while leaving all real public-production facts `UNKNOWN`.

Tracking issue: #238.

## Acceptance criteria

- [ ] A trusted-main manual workflow runs only on the dedicated `oteryn-staging` runner and `synology-staging` environment.
- [ ] The live stack proves exactly one running MariaDB, Redis, Canary, Platform, internal proxy and Gateway container for the expected Compose project.
- [ ] Platform, Gateway and legacy login remain loopback-only; game TCP is loopback or one exact private IPv4; MariaDB, Redis and internal proxy have no host-published ports.
- [ ] Platform and Gateway use exact `sha-<40 hex>` release tags and Canary uses an immutable digest reference.
- [ ] Required named volumes, mounts, restart policies, MariaDB health, Redis health/AOF and the last-good runtime snapshot are present.
- [ ] Existing application/Gateway/Canary health checks and all three Canary database privilege verifiers pass against the live stack.
- [ ] A streaming MariaDB restore drill restores Platform and Canary into temporary isolated databases, compares schema/row-count manifests, and removes all temporary state without uploading or retaining database contents.
- [ ] A sanitized exact-run artifact records only statuses, counts, durations and non-secret image/release identities with `classification: STAGING_PROVEN` and `production_environment_proven: false`.
- [ ] Remaining public-production gaps are recorded without promoting DNS/TLS/Cloudflare/WAF/mail/monitoring/DSM-backup/game-login facts.
- [ ] No router, NAT, DSM reverse proxy, public DNS, Cloudflare account, production, secret or external-repository action occurs.
- [ ] Temporary one-shot dispatch machinery is removed and the completed task is archived after live evidence is persisted.

## Ownership

```yaml
owned_paths:
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/one-shot-synology-production-target-preflight.yml
  - deploy/synology/.production-target-preflight-trigger
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/README.md
  - docs/operations/SYNOLOGY_PRODUCTION_TARGET_PREFLIGHT_EVIDENCE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-synology-production-target-preflight.md
  - docs/agents/tasks/archive/OTERYN-20260727-synology-production-target-preflight.md
modules:
  - Deployment
  - Operations
  - Security
  - Testing
  - AgentGovernance
dependencies:
  - existing Synology staging stack and dedicated runner
  - Issue 238 tracking the local live preflight
  - Issue 91 retaining the real production boundary
blockers: []
cross_repository_tasks: []
```

## Security and compatibility boundary

- Trust boundary: repository-reviewed trusted-main workflow -> dedicated Synology deployment runner -> local Docker socket and local staging containers.
- Authentication/authorization invariant: unchanged; Platform auth, MFA, RBAC and audit remain authoritative.
- Canary/login-server schema or session compatibility: unchanged; the drill copies the currently deployed schemas into temporary databases only.
- Rollback: temporary restore databases are dropped in an exit trap; no deployed schema or runtime image is changed.
- Secrets/production configuration: staging secrets are injected only through the GitHub Environment and are never written to evidence; no production-only configuration is used.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T16:05:00+02:00
head: f5aeb2e80d4692b3ee6309cc3454aa20697721f2
branch: ops/OTERYN-20260727-synology-production-target-preflight
pr: none
status: implementing
context_routes:
  - agent-governance
  - security
  - testing
  - deployment
  - operations
owned_paths:
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/one-shot-synology-production-target-preflight.yml
  - deploy/synology/.production-target-preflight-trigger
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/README.md
  - docs/operations/SYNOLOGY_PRODUCTION_TARGET_PREFLIGHT_EVIDENCE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-synology-production-target-preflight.md
  - docs/agents/tasks/archive/OTERYN-20260727-synology-production-target-preflight.md
proven:
  - main head f5aeb2e80d4692b3ee6309cc3454aa20697721f2 has no active task after edge-emulation archival
  - Issue 238 is open for the bounded local Synology production-target preflight
  - the existing stack keeps Platform Gateway and legacy login on loopback, exposes only optional private-LAN game TCP, and keeps MariaDB and Redis unpublished
  - the existing deployment already proves migrations, health checks, three Canary effective-grant verifiers and runtime-image rollback snapshots
  - Issue 91 remains open and requires direct evidence from a future real production environment
  - no open pull request overlaps the proposed preflight paths or intent
derived:
  - a live read-mostly host preflight plus temporary isolated restore drill can close the locally verifiable target-readiness gap without making a production claim
unknown:
  - exact currently deployed Platform Gateway and Canary image references until the live Synology workflow inspects them
  - whether the first live restore drill passes on the current Synology MariaDB dataset
conflicts: []
first_failure:
  marker: none
  evidence: implementation has not yet executed
rejected_hypotheses:
  - local Synology evidence can close Issue 91: public provider and production facts remain direct-environment requirements
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260727-synology-production-target-preflight.md
validation:
  - command: repository Synology deployment and production-boundary review
    result: PASS
    evidence: existing deployment package, health/rollback scripts and Production Go-Live Gate preserve staging-only classification
blockers: []
next_action: Implement the sanitized live Synology preflight script, trusted-main workflow, temporary dispatcher and evidence record.
```

## Notes

The live workflow may inspect non-secret container/image metadata and aggregate database counts only. It must not emit environment variables, credentials, private keys, database rows, dump bytes, private endpoint inventories or copied `.env` content.