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
search_first:
  - PR #387 public-domain validation report and findings
  - open tasks and PRs overlapping Synology deployment and public endpoint paths
  - canonical APP_URL requestless URL generation and deployment health checks
optional_reads:
  - PR #383
  - PR #385
  - PR #335
  - Issue #91
---

# OTERYN-20260801-public-domain-repair

## Goal

Repair the repository-owned public-domain defects proven by PR #387, prepare exact operator changes for unavailable edge infrastructure, and validate the repair candidate without weakening proxy trust, origin isolation, authentication controls or sensitive-response caching.

## Acceptance criteria

- [x] Canonical requestless Platform URLs use `https://oteryn.molehill.cloud` while loopback origins remain private.
- [x] Synology deployment inputs fail closed on a non-canonical public `APP_URL` for the public staging stack.
- [x] Bounded staging health checks verify canonical requestless URLs, Gateway endpoint identity, no cross-routing and sensitive login response cache controls.
- [x] Exact Cloudflare/DNS/Synology operator changes and rollback are documented without secrets.
- [ ] Focused and component repository validation pass on the exact repair head.
- [x] External edge and staging evidence are recorded only when explicit access and authorization exist.
- [x] `PRODUCTION_PROVEN` remains false until Issue #91 is completed.

## Ownership

```yaml
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/PublicCanonicalUrlTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
modules:
  - public-web
  - identity
  - game-gateway
  - edge-transport
  - synology-staging
dependencies:
  - PR #387 source validation package
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - Issue #91 production go-live gate
blockers:
  - Cloudflare, DNS and Synology operator access is unavailable in this session.
  - Explicit staging deployment authorization is absent.
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T08:52:00Z
session_id: chatgpt-20260801-public-domain-repair-001
policy_version: 2
phase: implementation_and_staging_verification
execution_mode: chat-github-connector
repository_mutation_authorization: PROVEN
external_mutation_scope_authorization: PROVEN
external_operator_access: UNKNOWN
staging_deployment_authorization: UNKNOWN
context_pressure: medium
decomposition_decision: split
branch: fix/OTERYN-20260801-public-domain-repair
head: f9306422ec3460736e11f924b23e765bd10c1d53
pr: 388
status: validating
context_routes:
  - agent-governance
  - security
  - auth-identity
  - api
  - testing
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/PublicCanonicalUrlTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
proven:
  - Live task-start main is 7dac56d3f3f4606be958c875f278edbe410e6b54.
  - PR #387 is open and draft at c8ca2fc995fbbc4a0f3c7268872d3843db950af8 with PUBLIC DOMAIN LAUNCH BLOCKED / FAIL.
  - PR #387 first public failure is gateway-public-tls-handshake-failure.
  - PR #335 owns only deploy/synology/compose.yml and deploy/synology/scripts/boot-repair.sh; those paths remain unchanged by PR #388.
  - No repair task, branch or PR owned this scope before this task created branch fix/OTERYN-20260801-public-domain-repair and draft PR #388.
  - Canonical roles remain WWW to loopback port 8000 and Gateway to loopback port 8080.
  - Last exact staging evidence used Platform and Gateway revision 6bfbc5f351758392d144baf0d2877a290ec69535 and Canary digest sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f.
  - The repair sets guarded public staging APP_URL to https://oteryn.molehill.cloud and rejects any other value.
  - The repair preserves loopback origin bindings and enables Secure cookies for the guarded public staging workflow.
  - The repair adds requestless login, password-reset and signed-route canonical-origin regression coverage.
  - The repair extends Synology health checks for exact Gateway identity, bounded invalid login, private no-store headers, canonical requestless URLs and negative cross-routing.
  - The sanitized external operator plan and rollback are recorded in docs/agents/reports/OTERYN-20260801-public-domain-repair.md.
  - No Cloudflare, DNS, Synology runtime, staging, production, Canary or PR #387 evidence mutation occurred.
derived:
  - Repository-owned canonical URL and verification defects are repaired independently of unavailable edge access.
  - The external Gateway TLS and Cloudflare policy failures cannot be completed with repository access alone.
unknown:
  - Exact currently externally deployed Platform, Gateway and Canary identities.
  - Effective Cloudflare certificate, WAF, Access, bot, redirect and HSTS configuration.
  - Usable Cloudflare, DNS and Synology operator access in this session.
  - Explicit approval for staging deployment of this candidate.
  - Exact supported native-client minimum TLS version.
conflicts:
  - The historical exact staging APP_URL is loopback HTTP while the canonical public application origin is HTTPS; PR #388 repairs the repository default but deployed state remains unverified.
first_failure:
  marker: gateway-public-tls-handshake-failure
  evidence: PR #387 runs 30690877286 and 30690957415 failed TLS negotiation for login.oteryn.molehill.cloud before HTTP
rejected_hypotheses:
  - PR #335 does not own the selected repair paths except excluded compose.yml; its changed-file list is limited to compose.yml and boot-repair.sh.
  - A broad proxy-trust change is not required; the existing exact Synology proxy trust remains unchanged.
changed_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/PublicCanonicalUrlTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
validation:
  - command: live repository, PR, branch and ownership preflight
    result: PASS
    evidence: main, PR #387, PR #335, repair branch/task/PR absence and canonical contracts verified
  - command: external mutation access and staging authorization check
    result: BLOCKED
    evidence: no Cloudflare, DNS or Synology operator session and no explicit staging deployment approval
  - command: Synology Production Target Preflight run 30692524819 on b939c06fa98d6f8d5b82a5374762b3cd8415a698
    result: PASS
    evidence: workflow completed successfully
  - command: Character Bazaar Staging Validation run 30692524873 on b939c06fa98d6f8d5b82a5374762b3cd8415a698
    result: PASS
    evidence: workflow completed successfully
  - command: Edge Security Emulation run 30692524820 on b939c06fa98d6f8d5b82a5374762b3cd8415a698
    result: PASS
    evidence: workflow completed successfully
  - command: Game Auth Ticket Concurrency run 30692524821 on b939c06fa98d6f8d5b82a5374762b3cd8415a698
    result: PASS
    evidence: workflow completed successfully
  - command: Agent Governance run 30692524845 on b939c06fa98d6f8d5b82a5374762b3cd8415a698
    result: FAIL
    evidence: checkpoint validator rejected one unsupported nested authorization mapping; this checkpoint removes it
  - command: CI, Phase 7 Production-Like Validation, Platform DB Outage Validation and Build Synology Staging Images
    result: NOT_RUN
    evidence: exact repaired checkpoint head has not completed the automatic workflows yet
deployment_evidence:
  - Last exact staging run 30669701842 is historical STAGING_PROVEN evidence only.
  - No staging deployment was authorized or executed for PR #388.
rollback:
  - Repository rollback is a PR revert or branch reset before merge.
  - Existing runtime image rollback remains available through the established Synology deployment procedure.
  - External rollback must restore the exact pre-change certificate, tunnel, WAF, Access, bot, redirect and HSTS snapshots; no external mutation has occurred.
blockers:
  - External edge repair and staging deployment cannot run without usable operator access and explicit staging authorization.
next_action: Complete exact-head repository CI, then stop with the external-access and staging-authorization blocker checkpointed.
```

## Report

`docs/agents/reports/OTERYN-20260801-public-domain-repair.md`

## Notes

No Cloudflare, DNS, Synology runtime, production, Canary or PR #387 evidence mutation is authorized through repository access alone. External changes require recorded current state and rollback before execution.
