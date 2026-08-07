---
task_id: OTERYN-20260801-public-domain-repair
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
search_first:
  - PR #387 public-domain validation
  - PRs #388, #392 and #396 repository/staging repair
  - Character Bazaar Staging Control run 30695167157 and artifact 8817085021
  - Cloudflare Tunnel/DNS apply run 30700054602
  - public post-apply revalidation run 30701140509
  - remaining-edge audits 30702383389 and 30702827344
optional_reads:
  - Issue #91
---

# OTERYN-20260801-public-domain-repair

## Goal

Repair the repository and staging public-domain defects, converge the canonical Cloudflare Tunnel/DNS contract, and complete the remaining public TLS/policy acceptance without weakening security boundaries.

## Acceptance criteria

- [x] Repository canonical URL, Secure-cookie and bounded Gateway checks are merged.
- [x] Exact source `3eb109b505f7d1c8718cffb823de6d9d5166717c` is deployed and `STAGING_PROVEN`.
- [x] Canonical Tunnel ingress entries are reconciled.
- [x] Both canonical proxied DNS records are current.
- [x] Post-apply public TLS/HTTP behavior is independently revalidated.
- [x] Trusted-main GET-only auditing exists for remaining Cloudflare edge controls.
- [x] Current token scope and token self-management capability are directly audited.
- [ ] Protected Cloudflare token is replaced with minimum remaining-edge read scopes.
- [ ] Exact certificate, challenge, redirect, Access, Bot and HSTS state is captured.
- [ ] Smallest evidence-supported edge repair is applied and publicly accepted.
- [ ] Controlled redacted password-recovery delivery passes.
- [x] `PRODUCTION_PROVEN` remains false until Issue #91 completes.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/SynologyStagingNetworkBoundaryTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-edge-revalidation.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
modules:
  - public-web
  - identity
  - game-gateway
  - edge-transport
  - synology-staging
dependencies:
  - Issue #91 production go-live gate
  - GitHub environment production-cloudflare
  - external Cloudflare account administrator for token rotation
blockers:
  - protected Cloudflare token lacks remaining-edge read scopes and cannot inspect or expand its own policy
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T11:30:00Z
status: blocked
phase: cloudflare_token_rotation_blocked
branch: none
head: none
pr: none
context_routes:
  - agent-governance
  - security
  - auth-identity
  - api
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
repository_mutation_authorization: PROVEN
external_mutation_scope_authorization: PROVEN
staging_deployment_authorization: PROVEN
proven:
  - PRs #388, #392 and #396 repaired repository-owned public-domain and staging boundaries.
  - Exact deployment run 30695167157 and artifact 8817085021 classify source 3eb109b505f7d1c8718cffb823de6d9d5166717c as STAGING_PROVEN.
  - Cloudflare audit 30699270139 and apply 30700054602 converged Tunnel/DNS; only the bounded Tunnel configuration changed.
  - Public revalidation 30701140509 artifact 8818850803 still proves Gateway TLS failure, WWW 403, no HTTP redirect and max-age=0 HSTS.
  - PR #406 merged trusted-main GET-only remaining-edge auditing as 5ea883c26dead9d58d363df1fb7909e3c399e206.
  - Live remaining-edge audit 30702383389 artifact 8819238641 proved permission_denied for certificate packs, Rulesets, Bot, Access and selected zone settings.
  - PR #411 merged trusted-main GET-only token-capability auditing as 63771e2565dd0d691c8229d97090c0d0fcceb9c3.
  - Live capability audit 30702827344 artifact 8819368872 proved permission_denied for token self-details and the permission-group catalog.
  - Account API Tokens Read and Account API Tokens Write are not proven; the current token cannot self-expand.
  - No Cloudflare mutation occurred in either remaining-edge audit.
  - The former task branch docs/OTERYN-20260801-cloudflare-token-blocker is terminal through PR #417 and no longer represents active implementation ownership; the task remains active only as an external waiting/blocker record.
derived:
  - Cloudflare API integration is working; the exact current blocker is token scope.
  - Tunnel/DNS is not the cause of the remaining TLS and policy failures.
  - External token replacement is required before autonomous read-audit and repair can continue.
unknown:
  - Exact certificate product and coverage state for login.oteryn.molehill.cloud.
  - Exact Cloudflare control producing WWW challenge pages.
  - Exact redirect, Access, Bot, WAF and HSTS resource identifiers.
  - Minimal corresponding write scopes until read audit succeeds.
  - Controlled password-recovery delivery result.
conflicts:
  - STAGING_PROVEN and Tunnel/DNS convergence coexist with directly failing public acceptance; neither may be promoted to PUBLIC_DOMAIN_LAUNCH_READY or PRODUCTION_PROVEN.
first_failure:
  marker: gateway-public-tls-handshake-failure
  evidence: PR #387 and post-apply run 30701140509 both fail Gateway TLS before HTTP.
rejected_hypotheses:
  - Repository or staging configuration remains the blocker; exact staging deployment passed.
  - Tunnel or DNS drift remains the blocker; live apply converged both.
  - Cloudflare integration is absent; authenticated audit/apply runs succeeded.
  - The current token can expand itself; live capability audit could not read its own policy or permission catalog.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
validation:
  - command: Character Bazaar Staging Control run 30695167157
    result: PASS
    evidence: exact deployment and artifact 8817085021
  - command: Cloudflare Tunnel/DNS apply run 30700054602
    result: PASS
    evidence: tunnel and both DNS names current after verification
  - command: public revalidation run 30701140509
    result: FAIL
    evidence: Gateway TLS, WWW 403, redirects and HSTS still fail
  - command: live remaining-edge audit run 30702383389
    result: PASS
    evidence: trusted GET-only audit completed and classified permission boundary
  - command: live token capability audit run 30702827344
    result: PASS
    evidence: trusted GET-only audit proved no token self-management access
  - command: remaining public acceptance with current token
    result: BLOCKED
    evidence: required edge API families are permission_denied
blockers:
  - An external Cloudflare account administrator must replace the protected production-cloudflare token with minimum remaining-edge read scopes without exposing the token in chat or Git.
next_action: Replace CLOUDFLARE_API_TOKEN in the protected GitHub environment, then rerun the existing marker-only trusted-main audit. Add only exact write scopes required by the resulting resource-level evidence, apply bounded changes, and rerun public acceptance.
```

## Reports

- `docs/agents/reports/OTERYN-20260801-public-domain-repair.md`
- `docs/agents/reports/OTERYN-20260801-public-edge-revalidation.md`
- `docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md`
