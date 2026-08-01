---
task_id: OTERYN-20260801-cloudflare-edge-audit
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
search_first:
  - PR #401 Cloudflare endpoint automation
  - PR #402 account token verification fix
  - Cloudflare audit run 30699270139
  - Cloudflare apply run 30700054602
  - public revalidation run 30701140509
optional_reads:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
---

# OTERYN-20260801-cloudflare-edge-audit

## Goal

Execute a protected read-only audit of the remaining Cloudflare edge controls after Tunnel and DNS convergence, without exposing the environment token or allowing trigger-branch code to run with it.

## Acceptance criteria

- [x] Audit implementation uses GET requests only.
- [x] Exact certificate coverage for `login.oteryn.molehill.cloud` is evaluated.
- [x] Redirect, Rulesets/WAF, Bot, Access, selected TLS settings and HSTS state are inspected when permissions allow.
- [x] Missing API permissions are classified without leaking credentials.
- [x] Pull-request validation uses deterministic mock API coverage.
- [x] Live audit code is checked out from trusted `main` under `pull_request_target`.
- [x] Trigger PR is restricted to one inert marker file.
- [ ] Implementation PR exact head passes all applicable workflows and is merged.
- [ ] Trigger PR executes the live audit and sanitized evidence is reviewed.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - tests/operations/cloudflare-oteryn-edge-audit/**
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - ops/triggers/cloudflare-edge-audit.md
modules:
  - operations
  - edge-security
dependencies:
  - GitHub environment production-cloudflare
  - merged Cloudflare endpoint automation
blockers: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T13:38:00Z
status: implementing
phase: read_only_edge_audit
branch: ops/OTERYN-20260801-cloudflare-edge-audit-v2
head: 430cbc20f95c6fa72fdfdb7f315409a423455abd
pr: 406
context_routes:
  - agent-governance
  - security
  - testing
owned_paths:
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - tests/operations/cloudflare-oteryn-edge-audit/mock_server.py
  - tests/operations/cloudflare-oteryn-edge-audit/run.sh
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - ops/triggers/cloudflare-edge-audit.md
repository_mutation_authorization: PROVEN
external_read_authorization: PROVEN
external_mutation_authorization: NOT_USED
proven:
  - Live Cloudflare audit 30699270139 and apply 30700054602 converged Tunnel/DNS.
  - Public revalidation 30701140509 proves Gateway TLS, WWW challenge, redirects and HSTS remain unresolved.
  - Existing endpoint automation cannot inspect certificates, Rulesets/WAF, Bot, Access, redirects or HSTS.
  - Local deterministic mock validation of the new audit passes.
  - Cloudflare Oteryn Edge Audit run 30701951772 passed on exact head f7cd61f834885113841b8804062ccb5d4477aff8.
derived:
  - A trusted-main read-only audit is the smallest safe next step before expanding Cloudflare mutation scope.
  - The trigger branch cannot modify code that receives the protected environment token.
unknown:
  - Whether the production-cloudflare token currently has read access to the remaining API families.
  - Whether exact-host certificate capability is available for the zone.
  - Which Cloudflare control produces the current public 403 challenge.
  - Current redirect, Access and HSTS rule ownership.
conflicts: []
first_failure:
  marker: active-task-checkpoint-schema-incomplete
  evidence: Agent Governance run 30701951794 job 91374392000 reported missing required checkpoint fields and unsupported status implementation.
rejected_hypotheses:
  - The audit implementation caused the governance failure; its own exact-head workflow passed before governance validation.
  - Live Cloudflare access occurred on the implementation PR; the live-audit job was correctly skipped.
changed_paths:
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - tests/operations/cloudflare-oteryn-edge-audit/mock_server.py
  - tests/operations/cloudflare-oteryn-edge-audit/run.sh
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - ops/triggers/cloudflare-edge-audit.md
validation:
  - command: bash tests/operations/cloudflare-oteryn-edge-audit/run.sh
    result: PASS
    evidence: local deterministic mock validation before repository submission
  - command: Cloudflare Oteryn Edge Audit run 30701951772
    result: PASS
    evidence: exact-head mock API, GET-only and sanitized-output validation
  - command: Agent Governance run 30701951794
    result: FAIL
    evidence: checkpoint schema failure only; corrected in subsequent heads
blockers: []
next_action: Re-run exact-head validation, merge the protected audit implementation, trigger one read-only live audit from trusted main, then design only the smallest evidence-supported repair.
```
