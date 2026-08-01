---
task_id: OTERYN-20260801-cloudflare-edge-audit
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
  - docs/operations/OTERYN_PUBLIC_EDGE_VALIDATION.md
search_first:
  - PR #427 Gateway hostname cutover
  - PR #433 trusted-main endpoint audit
  - PR #434 trusted-main endpoint apply
  - PR #435 post-apply endpoint audit
  - PR #436 post-migration public edge observer
  - Issue #91 endpoint operation evidence
optional_reads:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
---

# OTERYN-20260801-cloudflare-edge-audit

## Goal

Prove the post-migration Cloudflare certificate/policy state and public DNS/TLS/HTTP behavior for `oteryn.molehill.cloud` and `gateway.molehill.cloud`, then isolate the smallest remaining WAF/Bot remediation without exposing credentials, rule expressions, response bodies or cookies.

## Acceptance criteria

- [x] `gateway.molehill.cloud` is the canonical Game Gateway hostname.
- [x] Tunnel/DNS post-apply audit proves healthy/current canonical state and absent legacy DNS.
- [x] Advanced Certificate Manager is no longer required by the hostname design.
- [x] A trusted-main GET-only Cloudflare/public observer is implemented and validated offline.
- [x] The observer checks Gateway wildcard coverage, WAF/Bot/Access/settings, DNS, TLS 1.2/1.3, representative WWW routes, Gateway health/identity/login semantics, redirects and cross-routing.
- [x] Raw API responses, WAF expressions, country literals, response bodies and cookies are excluded from the published result.
- [x] The prior endpoint operation marker is reset to inert without weakening marker-only audit/apply authorization.
- [ ] Trusted-main observation is executed through an exact audit marker and publishes sanitized evidence to Issue #91.
- [ ] Remaining product failures are mapped to exact Cloudflare controls without speculative mutation.
- [ ] WAF/Bot apply occurs only after exact write scopes and reversible automation are validated.
- [ ] Native-client consumer configuration is updated through its separately controlled release path.
- [ ] `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until production acceptance passes.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-endpoint-main-operation.yml
  - .github/workflows/oteryn-public-edge-validation.yml
  - ops/triggers/cloudflare-oteryn-endpoints.md
  - ops/triggers/oteryn-public-edge-validation.md
  - scripts/operations/oteryn-public-edge-validation.py
  - scripts/operations/oteryn-public-edge-result.py
  - tests/operations/cloudflare-oteryn-endpoints/check-legacy-hostname.sh
  - tests/operations/cloudflare-oteryn-endpoints/check-main-operation-workflow.py
  - tests/operations/oteryn-public-edge-validation/**
  - docs/operations/OTERYN_PUBLIC_EDGE_VALIDATION.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
modules:
  - operations
  - edge-security
  - game-gateway
dependencies:
  - production-cloudflare GitHub environment
  - dedicated edge read token
  - GitHub-hosted runner public network
blockers:
  - none for merge or read-only production observation
  - WAF/Bot mutation requires Zone WAF Edit and Bot Management Edit
cross_repository_tasks:
  - native-client endpoint rollout remains a separate consumer update
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T18:34:00Z
head: 3414c06e1522262f33d93290eb1f63f3ee2659e2
branch: feat/OTERYN-20260801-public-edge-post-migration-validation
pr: 436
status: ready
policy_version: 2
task_kind: implementation
phase: public_edge_observer_validated
execution_mode: chat-github-connector
execution_reason: durable read-only validation, deterministic sanitization tests and exact-head repository gates
context_pressure: medium
decomposition_decision: phased
context_routes:
  - agent-governance
  - security
  - api
  - testing
owned_paths:
  - .github/workflows/cloudflare-oteryn-endpoint-main-operation.yml
  - .github/workflows/oteryn-public-edge-validation.yml
  - ops/triggers/cloudflare-oteryn-endpoints.md
  - ops/triggers/oteryn-public-edge-validation.md
  - scripts/operations/oteryn-public-edge-validation.py
  - scripts/operations/oteryn-public-edge-result.py
  - tests/operations/cloudflare-oteryn-endpoints/check-legacy-hostname.sh
  - tests/operations/cloudflare-oteryn-endpoints/check-main-operation-workflow.py
  - tests/operations/oteryn-public-edge-validation/**
  - docs/operations/OTERYN_PUBLIC_EDGE_VALIDATION.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
proven:
  - Trusted-main endpoint audit run 30712185284 proved healthy Tunnel, current canonical DNS, absent legacy DNS and no mutation.
  - Trusted-main apply run 30712337838 ended healthy/current with both canonical DNS records current, legacy DNS absent and no mutation.
  - Independent post-apply audit run 30712488508 reproduced healthy/current canonical state with mutation none.
  - Universal one-label wildcard design can cover gateway.molehill.cloud without Advanced Certificate Manager.
  - The public observer stores only bounded hashes, boolean body signals and allowlisted headers; Set-Cookie and raw response bodies are excluded.
  - Product acceptance FAIL remains a reported observation while collector execution failures still fail the workflow.
  - Operational audit/apply endpoint markers remain marker-only; only inert cleanup may accompany implementation changes.
  - Implementation head 3414c06e1522262f33d93290eb1f63f3ee2659e2 passed every triggered repository workflow.
derived:
  - Tunnel/DNS is no longer the first failure.
  - Remaining public failures, if present, can now be separated into certificate, WAF/Bot, browser protection, redirect or application behavior.
unknown:
  - Active certificate coverage currently reported for gateway.molehill.cloud after migration.
  - Public TLS handshake and Gateway HTTP behavior from an independent network.
  - Whether the country WAF block and Bot Fight Mode still prevent canonical requests.
  - Native-client consumer configuration location.
conflicts:
  - Endpoint control-plane state is current while public launch readiness remains unproven.
first_failure:
  marker: post-migration-public-edge-unvalidated
  evidence: Tunnel/DNS is proven current, but no current combined trusted-main Cloudflare/public observation exists for gateway.molehill.cloud
rejected_hypotheses:
  - Tunnel/DNS drift remains the blocker; three trusted-main operations ended current.
  - The retired multi-level hostname must be retained; canonical DNS and Tunnel use gateway.molehill.cloud.
  - Public validation requires Cloudflare mutation; certificate, policy and network behavior are observable read-only.
  - Resetting an operational marker requires weakening audit/apply isolation; inert cleanup is separately allowed while operational modes remain marker-only.
changed_paths:
  - .github/workflows/cloudflare-oteryn-endpoint-main-operation.yml
  - .github/workflows/oteryn-public-edge-validation.yml
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/operations/OTERYN_PUBLIC_EDGE_VALIDATION.md
  - ops/triggers/cloudflare-oteryn-endpoints.md
  - ops/triggers/oteryn-public-edge-validation.md
  - scripts/operations/oteryn-public-edge-result.py
  - scripts/operations/oteryn-public-edge-validation.py
  - tests/operations/cloudflare-oteryn-endpoints/check-legacy-hostname.sh
  - tests/operations/cloudflare-oteryn-endpoints/check-main-operation-workflow.py
  - tests/operations/oteryn-public-edge-validation/pass-fixture.json
  - tests/operations/oteryn-public-edge-validation/run.sh
validation:
  - command: Oteryn Public Edge Validation run 30712965258
    result: PASS
    evidence: deterministic PASS/FAIL fixtures, body/header sanitization and trusted workflow boundary passed on implementation head
  - command: Cloudflare Oteryn Endpoint Main Operation run 30712965238
    result: PASS
    evidence: inert cleanup and marker-only operational authorization checks passed
  - command: Cloudflare Oteryn Endpoints run 30712965236
    result: PASS
    evidence: fixed-scope endpoint automation, legacy-host guard and trusted result boundary passed
  - command: Cloudflare Oteryn Edge Audit run 30712965228
    result: PASS
    evidence: GET-only edge audit tests passed
  - command: Agent Governance run 30712965259
    result: PASS
    evidence: active task checkpoint and governance validation passed
  - command: CI run 30712965210
    result: PASS
    evidence: formatting, static analysis and application tests passed
  - command: Phase 7 Production-Like Validation run 30712965287
    result: PASS
    evidence: deployment, data, security, health, backup, restore, upgrade and rollback validation passed
  - command: Game Auth Ticket Concurrency run 30712965235
    result: PASS
    evidence: independent-process MariaDB concurrency proof passed
  - command: Edge Security Emulation run 30712965249
    result: PASS
    evidence: exact-head edge security emulation passed
  - command: Platform DB Outage Validation run 30712965294
    result: PASS
    evidence: fail-closed outage and recovery validation passed
blockers:
  - none for merge or trusted-main read-only observation
  - WAF/Bot apply remains blocked on Zone WAF Edit and Bot Management Edit
next_action: Merge PR #436, merge an exact public-edge audit marker, then inspect the sanitized Issue #91 result and implement only the smallest evidence-supported continuation.
```

## Report

`docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md`
