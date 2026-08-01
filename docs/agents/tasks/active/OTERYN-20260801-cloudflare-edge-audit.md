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
  - PR #425 fixed-scope apply preflight
  - marker PRs #423 and #426
  - runs 30708559130 and 30709108382
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
- [x] Certificate packs, certificate quota, zone settings, WAF rulesets, Bot Management and Access applications are readable.
- [x] Raw rule expressions, country literals and credentials are excluded from artifacts.
- [x] Exact certificate and broad country-block findings are recorded.
- [x] Advanced Certificate Manager quota is directly classified.
- [ ] Advanced Certificate Manager is enabled for the zone.
- [ ] Dedicated token receives only the minimum required zone-scoped Edit permissions.
- [ ] Fixed-scope apply automation passes deterministic and exact-head validation.
- [ ] Authorized live apply succeeds with rollback evidence.
- [ ] Public TLS/HTTP and controlled recovery acceptance pass.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - scripts/operations/cloudflare-edge-apply-preflight-audit.py
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
  - Advanced Certificate Manager is not enabled and the dedicated token remains read-only
```

## Context checkpoint

```yaml
checkpoint_version: 3
updated_at: 2026-08-01T16:56:00Z
session_id: chatgpt-20260801-cloudflare-edge-audit-005
policy_version: 2
status: blocked
phase: external_entitlement_and_write_scope
execution_mode: chat-github-connector
context_pressure: low
decomposition_decision: phased
branch: docs/OTERYN-20260801-cloudflare-edge-read-complete
source_main: ee38558a8420c8c32a8cfa92b69e60910e1695c5
pr: 424
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
  - PR #425 merged fixed-scope GET-only preflight as ee38558a8420c8c32a8cfa92b69e60910e1695c5.
  - Live run 30708559130 job 91391822768 completed with artifact 8821103628 and digest sha256:95fe01f1ebeec45aabad5c0e5c71e7cea866224b6e1f9648674949b508321128.
  - Live preflight run 30709108382 job 91393282575 completed with artifact 8821278907 and digest sha256:520bdbf591388ff30bba4cce232be413bab671ff040b6fb619e2c933d4553559.
  - Two Universal certificate packs exist and neither covers login.oteryn.molehill.cloud.
  - Advanced certificate quota is readable and reports allocated 0, used 0.
  - Access has eight applications and none targets either canonical Oteryn hostname.
  - Custom WAF ruleset 67ca2e19272a4c7d97c2a53681d0eb2f has one enabled broad block rule e0f91939eb494d4490d975498a9a9724.
  - The blocking expression fingerprint still matches the prior evidence.
  - The broad block uses ip.geoip.country with operator ne and has no host, path or method predicate.
  - Bot Fight Mode and JavaScript detections are enabled.
  - Browser Check is on, security level is high, Always Use HTTPS is on and minimum TLS is 1.3.
  - HSTS is enabled with max_age 0, includeSubDomains and preload.
  - No Cloudflare mutation occurred.
derived:
  - The country-based block is the direct owner of public 403 responses from regions outside its configured country.
  - Access is not the owner of the public block.
  - Gateway TLS cannot pass until Advanced Certificate Manager is enabled and an advanced certificate covers the canonical multi-level hostname.
  - The WAF repair must add an exact Oteryn hostname exception while preserving the country restriction for unrelated services.
  - Bot Fight Mode should be disabled for the machine/native-client Gateway because it is zone-wide and cannot be skipped by a normal custom rule.
unknown:
  - Whether Browser Check or security level remains a blocker after the exact WAF exception and Bot Fight Mode repair.
  - Whether TLS 1.2 is required by every supported native client.
conflicts:
  - Always Use HTTPS is on, but previous public HTTP probes received 403 before redirect due to the country-based block.
  - HSTS includes subdomains/preload while max_age 0 disables persistence because Gateway TLS is invalid.
first_failure:
  marker: advanced-certificate-entitlement-missing
  evidence: run 30709108382 artifact 8821278907 reports advanced allocated 0 and used 0
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
validation:
  - command: PR #420 exact-head workflow suite
    result: PASS
  - command: PR #422 exact-head workflow suite
    result: PASS
  - command: PR #425 exact-head workflow suite
    result: PASS
  - command: live permission-complete edge audit run 30708559130
    result: PASS
  - command: live fixed-scope apply preflight run 30709108382
    result: PASS
    evidence: certificate quota and exact blocking rule were readable; rule fingerprint matched; no mutation
blockers:
  - Enable Advanced Certificate Manager for molehill.cloud through Cloudflare billing/dashboard.
  - Add Zone WAF Edit, Bot Management Edit, Zone Settings Edit and SSL and Certificates Edit to the zone-scoped dedicated token.
next_action: Enable Advanced Certificate Manager and add the four zone-scoped Edit permissions to Oteryn Edge Audit; then implement and validate a guarded apply that creates an exact-host WAF exception, disables Bot Fight Mode, orders the advanced certificate and preserves rollback.
```

## Report

`docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md`
