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
- [x] `apply` requires the exact confirmation phrase `APPLY-OTERYN-CLOUDFLARE` and only reconciles the two canonical Oteryn hostnames.
- [x] Existing unrelated tunnel ingress rules and the existing final catch-all rule are preserved.
- [x] Existing unrelated DNS records are preserved; conflicting records fail closed.
- [x] Secrets and token values are never printed, committed or uploaded as artifacts.
- [x] Workflow/script configuration received focused deterministic validation before readiness.
- [x] Live audit completed before the separately authorized production apply.
- [x] Production apply completed with bounded tunnel-only mutation and post-apply verification.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - tests/operations/cloudflare-oteryn-endpoints/**
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/agents/tasks/archive/OTERYN-20260801-cloudflare-endpoint-automation.md
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

## Completion checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T12:37:10Z
status: completed
implementation_pr: 401
implementation_merge: aa2c263fb373215334234bb992223285ab34ea72
auth_fix_pr: 402
auth_fix_merge: ede1dfc44ae50da3e8d0b0b44d0fbe14f6c847dc
context_routes:
  - testing
  - security
proven:
  - Repository contract maps oteryn.molehill.cloud to http://127.0.0.1:8000 and login.oteryn.molehill.cloud to http://127.0.0.1:8080.
  - PR 401 merged the fixed-scope Cloudflare audit/apply workflow, fail-closed reconciler, deterministic mock tests and operator documentation.
  - PR 402 corrected account-owned cfat_ token verification to use the account-scoped Cloudflare endpoint while preserving user-token support.
  - Exact-head pull-request validation passed for the implementation and authentication fix before merge.
  - Live audit run 30699270139 completed successfully after the Cloudflare token permissions were corrected.
  - The successful live audit reported tunnel=drift, dns_www=current, dns_login=current and mutation=none.
  - Explicit owner-authorized apply run 30700054602 completed successfully from main with the exact confirmation phrase.
  - Apply run 30700054602 reported mode=apply, tunnel=current, dns_www=current, dns_login=current and mutation=tunnel.
  - The apply path re-read and verified tunnel and DNS state after mutation before returning success.
  - No DNS mutation was required; the only live mutation was the bounded remote tunnel configuration update.
  - No token or secret value was emitted by the workflow.
derived:
  - The two canonical Oteryn public hostnames now conform to the repository endpoint contract at the Cloudflare tunnel and DNS layers.
unknown:
  - End-to-end application availability behind the Synology origin was not exercised by this task and remains subject to deployment/runtime verification.
conflicts: []
first_failure:
  marker: account-owned cfat_ token was initially checked through the user-token verification endpoint
  evidence: live audit run 30698817353 failed read-only with HTTP 401 before tunnel or DNS access; repaired by PR 402
rejected_hypotheses:
  - The Cloudflare token itself was invalid: account-scoped token verification passed after the endpoint fix.
  - DNS required reconciliation: both canonical DNS records were already current in audit and apply.
  - Apply needed broad replacement or deletion: the successful run performed only mutation=tunnel and preserved current DNS.
validation:
  - command: Cloudflare Oteryn Endpoints PR validation run 30698592826
    result: PASS
    evidence: fixed-scope unit, mock API, audit, apply, secret-redaction and idempotency validation on the implementation head
  - command: Cloudflare Oteryn Endpoints PR validation run 30699040554
    result: PASS
    evidence: account-token verification fix validation on PR 402 head
  - command: Cloudflare Oteryn Endpoints run 30699270139, audit rerun job 91367838668
    result: PASS
    evidence: mode=audit tunnel=drift dns_www=current dns_login=current mutation=none
  - command: Cloudflare Oteryn Endpoints run 30700054602, manage job 91369383857
    result: PASS
    evidence: mode=apply tunnel=current dns_www=current dns_login=current mutation=tunnel with built-in post-apply verification
blockers:
  - none
next_action: none
```

## Notes

- Trust boundary: GitHub Actions receives the Cloudflare API token only inside the protected `production-cloudflare` environment and calls fixed Cloudflare v4 endpoints.
- Authentication/authorization invariant: repository dispatch inputs cannot select arbitrary Cloudflare resources.
- Rollback: the workflow performs no automatic deletion; restoration of a previous tunnel route value remains an explicit Cloudflare dashboard or reviewed API action.
- Production secrets remain outside Git and are not reproduced in this archived record.
- This task proves Cloudflare tunnel and DNS contract convergence only; it does not promote the application deployment or origin runtime to `PRODUCTION_PROVEN`.
