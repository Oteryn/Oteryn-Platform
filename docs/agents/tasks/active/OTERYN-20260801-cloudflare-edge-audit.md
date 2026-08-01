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
  - PR #420 dedicated edge-audit user token
  - PR #422 sanitized rule-scope evidence
  - marker PR #423
  - live audit run 30708559130 and artifact 8821103628
  - Cloudflare Tunnel/DNS apply run 30700054602
optional_reads:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - Issue #91
---

# OTERYN-20260801-cloudflare-edge-audit

## Goal

Identify and safely repair the Cloudflare edge controls blocking the canonical Oteryn WWW and Gateway hostnames without changing hostname responsibilities or unrelated zone configuration.

## Acceptance criteria

- [x] Trusted GET-only edge audit is merged.
- [x] Dedicated least-privilege user token is wired separately from the Tunnel/DNS account token.
- [x] Certificate packs, zone settings, WAF rulesets, Bot Management and Access applications are readable.
- [x] Raw rule expressions and credentials are excluded from artifacts.
- [x] Exact certificate and broad-block findings are recorded.
- [ ] Advanced Certificate Manager entitlement and certificate quota are proven.
- [ ] Dedicated token receives only the minimum required zone-scoped Edit permissions.
- [ ] Fixed-scope apply automation passes deterministic and exact-head validation.
- [ ] Authorized live apply succeeds with rollback evidence.
- [ ] Public TLS/HTTP and controlled recovery acceptance pass.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - scripts/operations/cloudflare-token-capability-audit.py
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
  - dedicated secret CLOUDFLARE_EDGE_AUDIT_TOKEN
  - Advanced Certificate Manager entitlement for multi-level Gateway hostname
blockers:
  - dedicated token is read-only and Advanced Certificate Manager entitlement is not yet proven
```

## Context checkpoint

```yaml
checkpoint_version: 2
updated_at: 2026-08-01T16:45:00Z
session_id: chatgpt-20260801-cloudflare-edge-audit-004
policy_version: 2
status: blocked
phase: write_scope_and_certificate_entitlement
execution_mode: chat-github-connector
context_pressure: low
decomposition_decision: phased
branch: docs/OTERYN-20260801-cloudflare-edge-read-complete
source_main: 4dec2825a9375040dcee01a5dde5426d102ffe35
pr: pending
context_routes:
  - agent-governance
  - security
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
repository_mutation_authorization: PROVEN
external_read_authorization: PROVEN
external_mutation_authorization: OWNER_INTENT_PROVEN_BUT_CREDENTIAL_NOT_CAPABLE
proven:
  - PR #420 merged dedicated user-token wiring as d4a3c0c56673ac1ff918f5be94d0b3be0bfe7ec3.
  - PR #422 merged sanitized rule-scope evidence as 4dec2825a9375040dcee01a5dde5426d102ffe35.
  - Live run 30708559130 job 91391822768 completed from trusted main with artifact 8821103628 and digest sha256:95fe01f1ebeec45aabad5c0e5c71e7cea866224b6e1f9648674949b508321128.
  - Two Universal certificate packs exist and neither covers login.oteryn.molehill.cloud.
  - Access has eight applications and none targets either canonical Oteryn hostname.
  - Custom WAF ruleset 67ca2e19272a4c7d97c2a53681d0eb2f has one enabled broad block candidate e0f91939eb494d4490d975498a9a9724.
  - Bot Fight Mode and JavaScript detections are enabled.
  - Browser Check is on, security level is high, Always Use HTTPS is on and minimum TLS is 1.3.
  - HSTS is enabled with max_age 0, includeSubDomains and preload.
  - No Cloudflare mutation occurred.
derived:
  - The broad custom block is the first evidence-supported owner of the public WWW 403 response.
  - Access is not the owner of the public block.
  - Gateway TLS cannot pass until a certificate covers the canonical multi-level hostname.
  - A precise exemption must preserve unrelated WAF behavior rather than delete the ruleset or disable the broad rule globally.
unknown:
  - Whether Advanced Certificate Manager is purchased and an advanced certificate quota is available.
  - Whether Browser Check or security level remains a blocker after the WAF exemption and Bot Fight Mode repair.
  - Whether TLS 1.2 is required by every supported native client.
conflicts:
  - Always Use HTTPS is on, but previous public HTTP probes received 403 before redirect due to another edge control.
  - HSTS includes subdomains/preload while max_age 0 deliberately disables persistence because Gateway TLS is invalid.
first_failure:
  marker: canonical-gateway-certificate-coverage-missing
  evidence: run 30708559130 artifact 8821103628
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
validation:
  - command: PR #420 exact-head workflow suite
    result: PASS
  - command: PR #422 exact-head workflow suite
    result: PASS
  - command: live permission-complete edge audit run 30708559130
    result: PASS
    evidence: all required zone reads and Access read succeeded; sanitized artifact uploaded
blockers:
  - Add Zone WAF Edit, Bot Management Edit, Zone Settings Edit and SSL and Certificates Edit to the zone-scoped dedicated token.
  - Prove or purchase Advanced Certificate Manager before ordering a certificate for login.oteryn.molehill.cloud.
next_action: Add the four zone-scoped Edit permissions to Oteryn Edge Audit and confirm Advanced Certificate Manager is active; then implement and validate the fixed-scope guarded apply without broad account write access.
```

## Report

`docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md`
