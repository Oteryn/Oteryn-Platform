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
- [ ] Trusted-main Cloudflare edge audit proves active wildcard coverage for the Gateway and current WAF/Bot/Access/settings state.
- [ ] Independent GitHub-hosted DNS/TLS/HTTP probes classify both canonical endpoints.
- [ ] Sanitized evidence is uploaded and a fixed allowlist result is published to Issue #91.
- [ ] Remaining product failures are mapped to exact Cloudflare controls without speculative mutation.
- [ ] WAF/Bot apply occurs only after exact write scopes and reversible automation are validated.
- [ ] Native-client consumer configuration is updated through its separately controlled release path.
- [ ] `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until all production acceptance passes.

## Ownership

```yaml
owned_paths:
  - .github/workflows/oteryn-public-edge-validation.yml
  - ops/triggers/oteryn-public-edge-validation.md
  - scripts/operations/oteryn-public-edge-validation.py
  - scripts/operations/oteryn-public-edge-result.py
  - tests/operations/oteryn-public-edge-validation/**
  - docs/operations/OTERYN_PUBLIC_EDGE_VALIDATION.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
modules:
  - operations
  - edge-security
  - game-gateway
dependencies:
  - production-cloudflare GitHub environment
  - dedicated edge read token
  - GitHub-hosted runner public network
blockers:
  - none for read-only validation
  - WAF/Bot mutation requires Zone WAF Edit and Bot Management Edit
cross_repository_tasks:
  - native-client endpoint rollout remains a separate consumer update
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T18:12:00Z
head: 44a018b3ad25bfad57651368ce535398a03927ec
branch: feat/OTERYN-20260801-public-edge-post-migration-validation
pr: none
status: implementing
policy_version: 2
task_kind: implementation
phase: post_migration_public_edge_validation
execution_mode: chat-github-connector
execution_reason: durable read-only validation, exact-head CI and trusted-main production observation
context_pressure: medium
decomposition_decision: phased
context_routes:
  - agent-governance
  - security
  - api
  - testing
owned_paths:
  - .github/workflows/oteryn-public-edge-validation.yml
  - ops/triggers/oteryn-public-edge-validation.md
  - scripts/operations/oteryn-public-edge-validation.py
  - scripts/operations/oteryn-public-edge-result.py
  - tests/operations/oteryn-public-edge-validation/**
  - docs/operations/OTERYN_PUBLIC_EDGE_VALIDATION.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
proven:
  - Trusted-main endpoint audit run 30712185284 proved healthy Tunnel, current canonical DNS, absent legacy DNS and no mutation.
  - Trusted-main apply run 30712337838 ended healthy/current with both canonical DNS records current, legacy DNS absent and no mutation.
  - Independent post-apply audit run 30712488508 reproduced healthy/current canonical state with mutation none.
  - Universal one-label wildcard design can cover gateway.molehill.cloud without Advanced Certificate Manager.
  - Existing Cloudflare edge collectors are GET-only and sanitize WAF expressions and credentials.
derived:
  - Tunnel/DNS is no longer the first failure.
  - Remaining public failures, if present, must be evaluated against certificate issuance/presentation, WAF/Bot policy, browser protections or application behavior.
unknown:
  - Active certificate coverage currently reported for gateway.molehill.cloud after migration.
  - Public TLS handshake and Gateway HTTP behavior from an independent network.
  - Whether the country WAF block and Bot Fight Mode still prevent canonical requests.
  - Native-client consumer configuration location.
conflicts:
  - Endpoint control-plane state is current while public launch readiness remains unproven.
first_failure:
  marker: post-migration-public-edge-unvalidated
  evidence: Tunnel/DNS is proven current, but no current combined Cloudflare/public observation exists for gateway.molehill.cloud
rejected_hypotheses:
  - Tunnel/DNS drift remains the blocker; three trusted-main operations ended current.
  - The retired multi-level hostname must be retained; canonical DNS and Tunnel now use gateway.molehill.cloud.
  - Public validation requires Cloudflare mutation; certificate, policy and network behavior can be observed read-only.
changed_paths:
  - .github/workflows/oteryn-public-edge-validation.yml
  - ops/triggers/oteryn-public-edge-validation.md
  - scripts/operations/oteryn-public-edge-validation.py
  - scripts/operations/oteryn-public-edge-result.py
  - tests/operations/oteryn-public-edge-validation/pass-fixture.json
  - tests/operations/oteryn-public-edge-validation/run.sh
  - docs/operations/OTERYN_PUBLIC_EDGE_VALIDATION.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
validation:
  - command: deterministic bounded collector tests
    result: NOT_RUN
    evidence: workflow validation will run after PR creation
  - command: trusted-main post-migration observation
    result: NOT_RUN
    evidence: requires merged exact audit marker after implementation reaches main
blockers:
  - none for implementation and read-only observation
next_action: Open the implementation PR, pass exact-head validation, merge, then merge an exact audit marker and inspect the sanitized Issue #91 result.
```

## Report

`docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md`
