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
updated_at: 2026-08-01T13:26:00Z
status: implementation
phase: read_only_edge_audit
repository_mutation_authorization: PROVEN
external_read_authorization: PROVEN
external_mutation_authorization: NOT_USED
proven:
  - Live Cloudflare audit 30699270139 and apply 30700054602 converged Tunnel/DNS.
  - Public revalidation 30701140509 proves Gateway TLS, WWW challenge, redirects and HSTS remain unresolved.
  - Existing endpoint automation cannot inspect certificates, Rulesets/WAF, Bot, Access, redirects or HSTS.
  - Local deterministic mock validation of the new audit passes.
unknown:
  - Whether the production-cloudflare token currently has read access to the remaining API families.
  - Whether exact-host certificate capability is available for the zone.
  - Which Cloudflare control produces the current public 403 challenge.
  - Current redirect, Access and HSTS rule ownership.
next_action: Merge the protected audit implementation, trigger one read-only live audit from trusted main, then design only the smallest evidence-supported repair.
```
