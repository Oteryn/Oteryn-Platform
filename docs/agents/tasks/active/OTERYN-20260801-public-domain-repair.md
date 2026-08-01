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

- [ ] Canonical requestless Platform URLs use `https://oteryn.molehill.cloud` while loopback origins remain private.
- [ ] Synology deployment inputs fail closed on a non-canonical public `APP_URL` for the public staging stack.
- [ ] Bounded staging health checks verify canonical requestless URLs, Gateway endpoint identity, no cross-routing and sensitive login response cache controls.
- [ ] Exact Cloudflare/DNS/Synology operator changes and rollback are documented without secrets.
- [ ] Focused and component repository validation pass on the exact repair head.
- [ ] External edge and staging evidence are recorded only when explicit access and authorization exist.
- [ ] `PRODUCTION_PROVEN` remains false until Issue #91 is completed.

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
updated_at: 2026-08-01T08:38:00Z
session_id: chatgpt-20260801-public-domain-repair-001
policy_version: 2
phase: implementation_and_staging_verification
execution_mode: chat-github-connector
authorization:
  repository_mutation: PROVEN
  cloudflare_dns_synology_scope: PROVEN_CONDITIONAL
  cloudflare_dns_synology_access: UNKNOWN
  staging_deployment: UNKNOWN
context_pressure: medium
decomposition_decision: split
branch: fix/OTERYN-20260801-public-domain-repair
head: 7dac56d3f3f4606be958c875f278edbe410e6b54
pr: none
status: implementing
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
  - Live main is 7dac56d3f3f4606be958c875f278edbe410e6b54.
  - PR #387 is open and draft at c8ca2fc995fbbc4a0f3c7268872d3843db950af8 with PUBLIC DOMAIN LAUNCH BLOCKED / FAIL.
  - PR #387 first failure is gateway-public-tls-handshake-failure.
  - PR #335 owns only deploy/synology/compose.yml and deploy/synology/scripts/boot-repair.sh; those paths are excluded from this task.
  - No existing repair task, branch or PR owns OTERYN-20260801-public-domain-repair.
  - Canonical roles are WWW to loopback port 8000 and Gateway to loopback port 8080.
  - Current staging deployment workflow defaults APP_URL to http://127.0.0.1:8000 while the canonical public root is https://oteryn.molehill.cloud.
  - Last exact staging evidence used Platform and Gateway revision 6bfbc5f351758392d144baf0d2877a290ec69535 and Canary digest sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f.
  - Repository writes are authorized for the exact claimed paths.
derived:
  - Repository-owned canonical URL and verification defects can be repaired independently of Cloudflare mutation.
  - The external Gateway TLS and Cloudflare policy failures cannot be completed with repository access alone.
unknown:
  - Exact currently externally deployed Platform, Gateway and Canary identities.
  - Effective Cloudflare certificate, WAF, Access, bot, redirect and HSTS configuration.
  - Usable Cloudflare, DNS and Synology operator access in this session.
  - Explicit approval for staging deployment of this candidate.
conflicts:
  - Exact staging APP_URL is loopback HTTP while the canonical public application origin is HTTPS.
first_failure:
  marker: gateway-public-tls-handshake-failure
  evidence: PR #387 runs 30690877286 and 30690957415 failed TLS negotiation for login.oteryn.molehill.cloud before HTTP
rejected_hypotheses:
  - PR #335 does not own the selected repair paths except excluded compose.yml; its changed-file list is limited to compose.yml and boot-repair.sh.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
validation:
  - command: live repository, PR, branch and ownership preflight
    result: PASS
    evidence: main, PR #387, PR #335, repair branch/task/PR absence and canonical contracts verified
  - command: external mutation access and staging authorization check
    result: BLOCKED
    evidence: no Cloudflare, DNS or Synology connector/operator session and no explicit staging deployment approval
  - command: focused implementation validation
    result: NOT_RUN
    evidence: implementation not yet committed
  - command: component and heavy validation
    result: NOT_RUN
    evidence: implementation not yet complete
deployment_evidence:
  - Last exact staging run 30669701842 is historical STAGING_PROVEN evidence only.
rollback:
  - Repository rollback is branch/PR revert before merge.
  - External rollback must restore the recorded pre-change Cloudflare/DNS/Synology configuration; no external mutation has occurred.
blockers:
  - External edge repair and staging deployment cannot run without usable operator access and explicit staging authorization.
next_action: Implement the canonical APP_URL contract, focused URL-generation regression test and bounded Synology health-check assertions on the claimed paths.
```

## Notes

No Cloudflare, DNS, Synology runtime, production, Canary or PR #387 evidence mutation is authorized through repository access alone. External changes require recorded current state and rollback before execution.
