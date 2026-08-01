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

- [ ] Workflow runs only from `main` and uses the protected `production-cloudflare` environment.
- [ ] `audit` performs token, account, zone, tunnel, ingress and DNS checks without mutation.
- [ ] `apply` requires an exact confirmation phrase and only upserts the two canonical Oteryn hostnames.
- [ ] Existing unrelated tunnel ingress rules and the existing final catch-all rule are preserved.
- [ ] Existing unrelated DNS records are preserved; conflicting records fail closed.
- [ ] Secrets and token values are never printed, committed or uploaded as artifacts.
- [ ] Workflow/script configuration receives focused deterministic validation before readiness.
- [ ] No live Cloudflare mutation occurs as part of this pull request.

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
updated_at: 2026-08-01T11:35:00Z
head: 2b0271d6704a70cd2ee43780e6f6ceb3c58d4789
branch: ops/cloudflare-oteryn-endpoints
pr: none
status: implementing
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
  - Cloudflare API supports token verification, tunnel/configuration reads and writes, and DNS record reads and writes through the documented v4 endpoints.
  - OWNER-CONFIRMED GitHub environment production-cloudflare contains CLOUDFLARE_API_TOKEN and the three required ID variables and permits main.
derived:
  - A fixed-hostname manual workflow can provide narrower authority than arbitrary Cloudflare API inputs.
unknown:
  - Whether the supplied token and identifiers authenticate successfully against the intended Cloudflare account, zone and tunnel.
  - Whether the selected tunnel is remotely managed and what its current live ingress and DNS state are.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-endpoint-automation.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation not yet created
blockers:
  - none
next_action: Open a draft pull request and implement the fixed-scope audit/apply workflow without executing it against Cloudflare.
```

## Notes

The workflow must not accept arbitrary hostnames, service URLs, zone IDs, account IDs or tunnel IDs as dispatch inputs. Live `apply` execution remains a separate explicit owner action after merge and review.