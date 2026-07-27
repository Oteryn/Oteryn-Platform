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
- [ ] A streaming MariaDB restore drill restores Platform and Canary into temporary isolated databases, compares exact deterministic dump digests and base-table counts, and removes all temporary state without uploading or retaining database contents.
- [ ] A sanitized exact-run artifact records only statuses, counts, durations and non-secret image/release identities with `classification: STAGING_PROVEN` and `production_environment_proven: false`.
- [ ] Remaining public-production gaps are recorded without promoting DNS/TLS/Cloudflare/WAF/mail/monitoring/DSM-backup/game-login facts.
- [ ] No router, NAT, DSM reverse proxy, public DNS, Cloudflare account, production, secret or external-repository action occurs.
- [ ] Temporary dispatch and observer machinery is removed and the completed task is archived after live evidence is persisted.

## Ownership

```yaml
owned_paths:
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/one-shot-synology-production-target-preflight.yml
  - .github/workflows/one-shot-synology-preflight-observer.yml
  - deploy/synology/.production-target-preflight-trigger
  - deploy/synology/.production-target-preflight-observer-trigger
  - deploy/synology/scripts/production-target-preflight.sh
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
- Rollback: temporary restore databases are dropped in a Python `finally` block; no deployed schema or runtime image is changed.
- Secrets/production configuration: staging secrets are injected only through the GitHub Environment and are never written to evidence; no production-only configuration is used.
- Observer boundary: the temporary observer has Actions read and Issues write only; it cannot dispatch, use the Synology runner, access NAS/Docker or read environment secrets.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T16:44:00+02:00
head: a9479f66af3b684f7caf3a85d1f66cbaf860d1bf
branch: ops/OTERYN-20260727-synology-preflight-observer
pr: 243
status: observing
context_routes:
  - agent-governance
  - security
  - testing
  - deployment
  - operations
owned_paths:
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/one-shot-synology-production-target-preflight.yml
  - .github/workflows/one-shot-synology-preflight-observer.yml
  - deploy/synology/.production-target-preflight-trigger
  - deploy/synology/.production-target-preflight-observer-trigger
  - deploy/synology/scripts/production-target-preflight.sh
  - docs/operations/SYNOLOGY_PRODUCTION_TARGET_PREFLIGHT_EVIDENCE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-synology-production-target-preflight.md
  - docs/agents/tasks/archive/OTERYN-20260727-synology-production-target-preflight.md
proven:
  - PR 239 merged as 50d917acd7fde333f0e74757ec1ced70e30c53de after all eight exact-head workflows passed
  - trusted-main commit contains the bounded live preflight and one-shot dispatcher with the activation marker
  - Issue 238 remains open and has no final dispatcher PASS or FAIL comment yet
  - PR 243 adds a read-only observer that queries only GitHub Actions metadata and posts a bounded status snapshot
  - Issue 91 remains open and requires direct evidence from a future real production environment
derived:
  - the absence of an Issue 238 result comment is not sufficient to distinguish a queued runner from dispatcher failure
  - the observer can resolve current Actions state without creating another live preflight or accessing the NAS
unknown:
  - current one-shot dispatcher and live workflow run identities/statuses until the observer executes
  - exact currently deployed Platform Gateway and Canary image references until the live Synology workflow succeeds
  - whether the live restore drill passes on the current Synology MariaDB dataset
conflicts: []
first_failure:
  marker: preflight-script-syntax
  evidence: Synology Production Target Preflight run 30274168248 failed during bash syntax validation before any live job; malformed backtick quoting was replaced by Python streaming digest verification
rejected_hypotheses:
  - silence in Issue 238 proves success or failure: only explicit workflow state or artifact evidence is authoritative
  - the observer mutates Synology: it has no Synology runner, environment or dispatch permission
  - local Synology evidence can close Issue 91: public provider and production facts remain direct-environment requirements
changed_paths:
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/one-shot-synology-production-target-preflight.yml
  - .github/workflows/one-shot-synology-preflight-observer.yml
  - deploy/synology/.production-target-preflight-trigger
  - deploy/synology/scripts/production-target-preflight.sh
  - docs/operations/SYNOLOGY_PRODUCTION_TARGET_PREFLIGHT_EVIDENCE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-synology-production-target-preflight.md
validation:
  - command: PR 239 exact-head required workflows
    result: PASS
    evidence: all eight required workflows passed on f4aaa3426f680f593be175ebd332ccf9769a4814
  - command: PR 239 squash merge
    result: PASS
    evidence: trusted-main activation commit 50d917acd7fde333f0e74757ec1ced70e30c53de
blockers: []
next_action: Pass PR 243 checks, merge the read-only observer, trigger its bounded snapshot and use that state to continue the live preflight lifecycle.
```

## Notes

The live workflow may inspect non-secret container/image metadata and aggregate database counts only. It must not emit environment variables, credentials, private keys, database rows, dump bytes, private endpoint inventories or copied `.env` content.