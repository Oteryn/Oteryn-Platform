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
  - final-head GitHub checks pending
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T11:56:00Z
head: f0c2dc1d1517a59e00cb2395eebb6cc3fd3acffe
branch: ops/cloudflare-oteryn-endpoints
pr: 401
status: validating
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
  - Deterministic local mock validation proves audit performs zero mutations, apply performs only one tunnel PUT plus bounded canonical DNS upserts, and a second apply is idempotent.
  - The workflow has no arbitrary Cloudflare resource inputs and no live Cloudflare action can run from a pull-request event.
  - GitHub run 30698413666 failed before functional validation because the Contents API stored the reconciler without an executable mode and the test invoked it directly.
  - Commit f0c2dc1d1517a59e00cb2395eebb6cc3fd3acffe repairs the runner-portability defect by invoking the reconciler explicitly through bash in every integration-test call.
derived:
  - The merged workflow will expose a narrower and more reviewable mutation path than manually supplying arbitrary Cloudflare API requests.
unknown:
  - Whether the supplied token and identifiers authenticate successfully against the intended live Cloudflare account, zone and tunnel.
  - Whether the selected live tunnel is remotely managed and what its current ingress and DNS drift are.
conflicts: []
first_failure:
  marker: Cloudflare Oteryn Endpoints validate step exited 126 with Permission denied
  evidence: run 30698413666 job 91365080250 on head c61af893cc0f6c8f1086c7e2213619fdb6bdf45a
rejected_hypotheses:
  - A tunnel PUT must overwrite unrelated routes: the reconciler preserves top-level configuration, unrelated ingress order and the existing final catch-all, with deterministic regression coverage.
  - The reconciler must rely on an executable Git file mode: workflow and tests can invoke the checked-in shell source explicitly through bash.
changed_paths:
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-endpoint-automation.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - tests/operations/cloudflare-oteryn-endpoints/mock_cloudflare.py
  - tests/operations/cloudflare-oteryn-endpoints/run.sh
validation:
  - command: bash tests/operations/cloudflare-oteryn-endpoints/run.sh
    result: PASS
    evidence: local deterministic unit and mock-API audit/apply/idempotency suite before GitHub runner execution
  - command: ruby -e 'require "yaml"; YAML.load_file(ARGV[0])' .github/workflows/cloudflare-oteryn-endpoints.yml
    result: PASS
    evidence: workflow YAML parsed successfully in the authored package
  - command: Cloudflare Oteryn Endpoints run 30698413666 job 91365080250
    result: FAIL
    evidence: direct test execution of non-executable script failed before functional checks; root cause repaired in f0c2dc1d1517a59e00cb2395eebb6cc3fd3acffe
  - command: GitHub Actions on PR 401 repaired final head
    result: NOT_RUN
    evidence: checks pending after portability repair
blockers:
  - final-head GitHub checks pending
next_action: Inspect every path-applicable GitHub check on PR 401 repaired final head and fix the first root-cause failure, or mark the task ready if all pass.
```

## Notes

- Trust boundary: GitHub Actions receives one Cloudflare API token only inside the `production-cloudflare` environment and calls fixed Cloudflare v4 endpoints.
- Authentication/authorization invariant: only the protected environment secret authenticates API calls; repository dispatch inputs cannot select arbitrary Cloudflare resources.
- Canary/schema/session compatibility: unchanged.
- Rollback: no automatic delete exists; a partial bounded apply is audited and converged by rerun, while immediate value restoration remains a Cloudflare dashboard action.
- Production-only configuration: the token and IDs stay outside Git; no secret value is read back or recorded in the task, PR, logs or artifacts.
- Live `apply` execution remains a separate explicit owner action after merge and review.