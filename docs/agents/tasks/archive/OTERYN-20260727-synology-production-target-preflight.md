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
  - deploy/synology/scripts/production-target-preflight.sh
  - .github/workflows/synology-production-target-preflight.yml
  - docs/operations/SYNOLOGY_PRODUCTION_TARGET_PREFLIGHT_EVIDENCE.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
search_first:
  - live Issue 238 result and exact workflow evidence
  - merged implementation PR and cleanup state
  - production-only facts that remain UNKNOWN
optional_reads: []
---

# OTERYN-20260727-synology-production-target-preflight

## Goal

Add and execute a deterministic production-target preflight against the existing local Synology staging stack, proving every safe locally verifiable host/runtime, storage, binding, health, least-privilege, persistence, restore-drill and rollback-readiness boundary without exposing the service publicly or claiming production verification.

Tracking issue: #238.

## Acceptance criteria

- [x] A trusted-main manual workflow ran only on the dedicated `oteryn-staging` runner and `synology-staging` environment.
- [x] The live stack proved exactly one running MariaDB, Redis, Canary, Platform, internal proxy and Gateway container for the expected Compose project.
- [x] Platform, Gateway and legacy login remained loopback-only; game TCP used one exact private/loopback IPv4; MariaDB, Redis and internal proxy had no host-published ports.
- [x] Platform and Gateway used exact matching `sha-<40 hex>` release tags and Canary used an immutable digest reference.
- [x] Required named volumes, mounts, restart policies, MariaDB health, Redis health/AOF and the last-good runtime snapshot were present.
- [x] Application/Gateway/Canary health checks and all three Canary database privilege verifiers passed against the live stack.
- [x] A streaming MariaDB restore drill restored Platform and Canary into temporary isolated databases, compared deterministic dump digests and base-table counts, and removed temporary state without uploading or retaining database contents.
- [x] A sanitized exact-run artifact recorded only statuses, counts, duration and non-secret image/release identities with `classification: STAGING_PROVEN` and `production_environment_proven: false`.
- [x] Remaining public-production gaps were recorded without promoting DNS/TLS/Cloudflare/WAF/mail/monitoring/DSM-backup/game-login facts.
- [x] No router, NAT, DSM reverse proxy, public DNS, Cloudflare account, production, secret or external-repository action occurred.
- [x] The temporary one-shot dispatcher and trigger are removed by the closeout PR and this completed task record is archived.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260727-synology-production-target-preflight.md
modules:
  - Deployment
  - Operations
  - Security
  - Testing
  - AgentGovernance
dependencies:
  - Issue 91 retaining the real production boundary
blockers: []
cross_repository_tasks: []
```

## Security and compatibility boundary

- The trusted workflow ran from reviewed `main` on the dedicated local Synology runner.
- Platform authentication, confirmed MFA, exact RBAC and audit behavior were not changed.
- Source Platform and Canary databases were read through a single-transaction dump stream only; temporary restore databases were isolated and dropped in a `finally` block.
- No dump file, row, secret, environment variable, private key, copied `.env` or private endpoint inventory entered the artifact or repository.
- No deployed schema, runtime image, router, DSM proxy, DNS, Cloudflare or external repository was modified.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T16:58:00+02:00
head: 50d917acd7fde333f0e74757ec1ced70e30c53de
branch: docs/OTERYN-20260727-synology-preflight-closeout
pr: none
status: ready
context_routes:
  - agent-governance
  - security
  - testing
  - deployment
  - operations
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260727-synology-production-target-preflight.md
proven:
  - PR 239 merged as 50d917acd7fde333f0e74757ec1ced70e30c53de after all eight required exact-head workflows passed
  - trusted-main Synology Production Target Preflight run 30275482522 completed successfully on workflow source SHA 50d917acd7fde333f0e74757ec1ced70e30c53de
  - sanitized artifact synology-production-target-preflight-evidence-30275482522 has digest sha256:b54ec5fc619201685fe792328dd9682e958b07f41ab6b5c2f9d6f255b1e2a704
  - inspected Compose project was oteryn-staging with deployed Platform and Gateway release 415aa3febd04c8d9c61082d4a7451352bf084013 and Canary digest sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f
  - container singleton state restart policies private network host binding restrictions unpublished MariaDB and Redis immutable images named volumes health effective grants Redis AOF ACL and rollback snapshot checks all passed
  - the isolated streaming restore drill passed for 34 Platform base tables and 59 Canary base tables in 717610 ms and removed temporary databases
  - the artifact records classification STAGING_PROVEN and production_environment_proven false
  - the current local runtime uses file sessions file cache synchronous queue and array non-delivery mail
  - PR 243 was closed without merge after the original dispatcher posted its authoritative success evidence
  - no production router NAT DSM reverse proxy public DNS Cloudflare account secret or external-repository action occurred
derived:
  - all repository-owned and currently locally verifiable Synology production-target preflight work is complete
  - the measured restore duration is staging dataset evidence only and is not a production RTO or RPO
  - Issue 91 remains the sole real Production Go-Live execution tracker
unknown: []
conflicts: []
first_failure:
  marker: preflight-script-syntax
  evidence: initial static run 30274168248 rejected malformed SQL quoting before any live job; the implementation was replaced with a reviewed Python streaming digest restore drill and all later exact-head and live checks passed
rejected_hypotheses:
  - the first failure involved the NAS or database: the initial live job was skipped before any Synology action
  - local file sessions cache synchronous queue or array mail prove a final production topology: they are recorded current local-target facts only
  - the 717610 ms staging restore measurement is a production RTO or RPO: no such inference is permitted
  - local Synology evidence closes Issue 91: direct real-environment public production evidence remains mandatory
changed_paths:
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/one-shot-synology-production-target-preflight.yml
  - deploy/synology/.production-target-preflight-trigger
  - deploy/synology/scripts/production-target-preflight.sh
  - docs/operations/SYNOLOGY_PRODUCTION_TARGET_PREFLIGHT_EVIDENCE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-synology-production-target-preflight.md
  - docs/agents/tasks/archive/OTERYN-20260727-synology-production-target-preflight.md
validation:
  - command: PR 239 exact-head required workflows
    result: PASS
    evidence: Synology preflight static, CI, Governance, Synology image/package validation, Phase 7, Edge, concurrency and DB outage all passed on final head f4aaa3426f680f593be175ebd332ccf9769a4814
  - command: trusted-main Synology Production Target Preflight 30275482522
    result: PASS
    evidence: both static-validation and live-preflight jobs passed and the sanitized artifact was uploaded then removed from the runner
  - command: sanitized artifact inspection
    result: PASS
    evidence: one JSON file only; all required PASS values, exact non-secret image identities, 34 and 59 table counts, 717610 ms duration and production_environment_proven false
blockers: []
next_action: Preserve this archived record as completion evidence.
```

## Notes

The permanent manual workflow and preflight script remain available for future local Synology re-validation. The one-shot dispatcher and trigger are intentionally removed after the successful live run.