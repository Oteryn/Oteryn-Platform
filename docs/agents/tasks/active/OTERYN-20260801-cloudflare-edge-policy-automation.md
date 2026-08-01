---
task_id: OTERYN-20260801-cloudflare-edge-policy-automation
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
search_first:
  - PR #407 post-Cloudflare public validation
  - Cloudflare endpoint audit 30699270139 and apply 30700054602
  - active Cloudflare or public-domain tasks and PRs
optional_reads:
  - PR #401
  - PR #402
  - PR #403
  - Issue #91
---

# OTERYN-20260801-cloudflare-edge-policy-automation

## Goal

Add a fail-closed, read-only Cloudflare audit path that identifies the exact certificate, challenge, redirect, Access, bot-management and HSTS state affecting only `oteryn.molehill.cloud` and `login.oteryn.molehill.cloud`, without mutating the live zone or exposing unrelated configuration.

## Acceptance criteria

- [ ] The workflow runs from pull requests only for deterministic validation and from `main` only for live audit.
- [ ] Live audit uses the protected `production-cloudflare` environment and performs no write request.
- [ ] Token/account/zone identity is verified without printing secrets.
- [ ] Certificate packs, SSL verification and Total TLS state are sanitized and classified for both canonical hosts.
- [ ] Bot Management, relevant zone settings, Access applications and relevant Rulesets are audited with permission failures recorded explicitly.
- [ ] Unrelated hostnames, rule expressions, application domains and identifiers are redacted or hashed.
- [ ] Deterministic tests prove read-only behavior, canonical-host filtering and secret redaction.
- [ ] The first live audit is reviewed before any apply-capable design is authorized.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - tests/operations/cloudflare-oteryn-edge-audit/**
  - docs/operations/CLOUDFLARE_EDGE_POLICY_AUDIT.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-policy-automation.md
modules:
  - operations
  - ci
  - edge-security
dependencies:
  - GitHub environment production-cloudflare
  - merged Cloudflare endpoint automation PRs #401 and #402
  - PR #407 public evidence package
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T13:45:00Z
session_id: chatgpt-20260801-cloudflare-edge-policy-001
session_role: implementer
policy_version: 2
phase: implement
execution_mode: chat-github-connector
execution_reason: bounded workflow, script, tests and documentation can be authored through repository connectors with local syntax validation
context_pressure: medium
context_growth: stable
decomposition_decision: phased
branch: ops/OTERYN-20260801-cloudflare-edge-policy-automation
head: de949075d14ebecc57423237b9330d865da28645
pr: none
status: implementing
context_routes:
  - agent-governance
  - security
  - testing
owned_paths:
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - tests/operations/cloudflare-oteryn-edge-audit/**
  - docs/operations/CLOUDFLARE_EDGE_POLICY_AUDIT.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-policy-automation.md
proven:
  - Cloudflare endpoint apply run 30700054602 made the canonical Tunnel entries and DNS records current.
  - PR #407 run 30701999967 still proves Gateway TLS failure, WWW Cloudflare 403 interstitials, missing HTTP redirects and HSTS max-age=0 after that apply.
  - The existing endpoint reconciler intentionally does not manage certificates, Bot/WAF/Access policy, redirect rules or HSTS.
  - Cloudflare exposes read APIs for certificate packs, SSL verification, Total TLS, Bot Management, zone settings, Rulesets and Access applications.
derived:
  - A separate read-only audit is required before any safe edge-policy mutation can be designed.
unknown:
  - Whether the existing production-cloudflare token has all required read permissions.
  - Which exact certificate product, rule, setting or Access application causes each public failure.
conflicts: []
first_failure:
  marker: gateway-public-tls-handshake-failure
  evidence: PR #407 run 30701999967 after successful Tunnel/DNS apply
rejected_hypotheses:
  - Tunnel or DNS drift is the remaining cause: run 30700054602 reconciled both and PR #407 reproduced the failures.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-policy-automation.md
validation:
  - command: implementation validation
    result: NOT_RUN
    evidence: task initialized before file creation
blockers:
  - none
next_action: Implement the fixed-scope read-only audit workflow, sanitized collector and deterministic tests.
```

## Notes

This task authorizes audit tooling only. It does not authorize certificate ordering, Total TLS changes, WAF/Bot/Access changes, redirects, HSTS mutation or any other live write.
