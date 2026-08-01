---
task_id: OTERYN-20260801-cloudflare-endpoint-automation
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
search_first:
  - active Cloudflare, public-domain, DNS, tunnel and deployment tasks or pull requests
optional_reads: []
---

# OTERYN-20260801-cloudflare-endpoint-automation

## Goal

Add a fail-closed, manually dispatched GitHub Actions workflow that can audit and narrowly reconcile the two canonical Oteryn Cloudflare Tunnel hostnames and their DNS records without exposing credentials or overwriting unrelated tunnel routes.

## Acceptance criteria

- [x] Workflow runs only from `main` and uses the protected `production-cloudflare` environment.
- [x] `audit` performs token, account, zone, tunnel, ingress and DNS checks without mutation.
- [x] `apply` requires an exact confirmation phrase and only upserts the two canonical Oteryn hostnames.
- [x] Existing unrelated tunnel ingress rules and the existing final catch-all rule are preserved.
- [x] Existing unrelated DNS records are preserved; conflicting records fail closed.
- [x] Secrets and token values are never printed, committed or uploaded as artifacts.
- [x] Workflow/script configuration receives focused deterministic validation before readiness.
- [x] No live Cloudflare mutation occurs as part of this pull request.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - tests/operations/cloudflare-oteryn-endpoints/**
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-endpoint-automation.md
modules:
  - operations
  - ci
  - edge-security
dependencies:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - GitHub environment production-cloudflare
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T12:03:00Z
head: f0e605945137e015129249a4253d1327eb05881d
branch: ops/cloudflare-oteryn-endpoints
pr: 401
status: ready
context_routes:
  - testing
  - security
owned_paths:
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - tests/operations/cloudflare-oteryn-endpoints/**
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-endpoint-automation.md
proven:
  - Repository contract maps oteryn.molehill.cloud to http://127.0.0.1:8000 and login.oteryn.molehill.cloud to http://127.0.0.1:8080.
  - Cloudflare API supports token verification, remote tunnel configuration reads and writes, and DNS record reads and writes through documented v4 endpoints.
  - OWNER-CONFIRMED GitHub environment production-cloudflare contains CLOUDFLARE_API_TOKEN and the three required ID variables and permits main.
  - PR 401 contains only the six declared workflow, script, test, operations-documentation and task-record paths.
  - Deterministic mock validation proves audit performs zero mutations, apply performs only one tunnel PUT plus bounded canonical DNS upserts, and a second apply is idempotent.
  - The workflow has no arbitrary Cloudflare resource inputs and no live Cloudflare action can run from a pull-request event.
  - GitHub run 30698413666 exposed a runner-portability defect before functional validation; commit f0c2dc1d1517a59e00cb2395eebb6cc3fd3acffe repaired it by invoking the reconciler through bash.
  - Every path-applicable workflow passed on exact repaired head f0e605945137e015129249a4253d1327eb05881d.
derived:
  - The merged workflow will expose a narrower and more reviewable mutation path than manually supplying arbitrary Cloudflare API requests.
unknown:
  - Whether the supplied token and identifiers authenticate successfully against the intended live Cloudflare account, zone and tunnel.
  - Whether the selected live tunnel is remotely managed and what its current ingress and DNS drift are.
conflicts: []
first_failure:
  marker: Cloudflare Oteryn Endpoints validate step exited 126 with Permission denied
  evidence: run 30698413666 job 91365080250 on head c61af893cc0f6c8f1086c7e2213619fdb6bdf45a; repaired in f0c2dc1d1517a59e00cb2395eebb6cc3fd3acffe
rejected_hypotheses:
  - A tunnel PUT must overwrite unrelated routes: the reconciler preserves top-level configuration, unrelated ingress order and the existing final catch-all, with deterministic regression coverage.
  - The reconciler must rely on an executable Git file mode: workflow and tests invoke the checked-in shell source explicitly through bash.
changed_paths:
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-endpoint-automation.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - tests/operations/cloudflare-oteryn-endpoints/mock_cloudflare.py
  - tests/operations/cloudflare-oteryn-endpoints/run.sh
validation:
  - command: Cloudflare Oteryn Endpoints run 30698492713
    result: PASS
    evidence: fixed-scope unit, mock API, audit, apply, secret-redaction and idempotency validation passed
  - command: Agent Governance run 30698492707
    result: PASS
    evidence: exact head f0e605945137e015129249a4253d1327eb05881d
  - command: CI run 30698492733
    result: PASS
    evidence: exact head f0e605945137e015129249a4253d1327eb05881d
  - command: Edge Security Emulation run 30698492736
    result: PASS
    evidence: exact head f0e605945137e015129249a4253d1327eb05881d
  - command: Platform DB Outage Validation run 30698492711
    result: PASS
    evidence: exact head f0e605945137e015129249a4253d1327eb05881d
  - command: Game Auth Ticket Concurrency run 30698492693
    result: PASS
    evidence: exact head f0e605945137e015129249a4253d1327eb05881d
  - command: Phase 7 Production-Like Validation run 30698492721
    result: PASS
    evidence: exact head f0e605945137e015129249a4253d1327eb05881d
blockers:
  - none
next_action: Review and merge PR 401, then run the merged workflow from main in audit mode before considering any apply operation.
```

## Notes

- Trust boundary: GitHub Actions receives one Cloudflare API token only inside the `production-cloudflare` environment and calls fixed Cloudflare v4 endpoints.
- Authentication/authorization invariant: only the protected environment secret authenticates API calls; repository dispatch inputs cannot select arbitrary Cloudflare resources.
- Canary/schema/session compatibility: unchanged.
- Rollback: no automatic delete exists; a partial bounded apply is audited and converged by rerun, while immediate value restoration remains a Cloudflare dashboard action.
- Production-only configuration: the token and IDs stay outside Git; no secret value is read back or recorded in the task, PR, logs or artifacts.
- Live `apply` execution remains a separate explicit owner action after merge and review.