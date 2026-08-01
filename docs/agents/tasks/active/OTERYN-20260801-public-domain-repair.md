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
  - merged PRs #388 and #392
  - Character Bazaar Staging Control runs 30693873142 and 30694481769
optional_reads:
  - PR #383
  - PR #385
  - PR #335
  - Issue #91
---

# OTERYN-20260801-public-domain-repair

## Goal

Repair the repository-owned public-domain defects proven by PR #387, deploy the exact repair through Marketplace-aware Synology staging, and retain a reversible operator plan for unavailable edge infrastructure without weakening security boundaries.

## Acceptance criteria

- [x] Requestless Platform URLs use `https://oteryn.molehill.cloud` while origins remain loopback-only.
- [x] Public staging rejects an unexpected full deployment `APP_URL`.
- [x] Partial Marketplace state loads without requiring deployment-only keys.
- [x] Marketplace Platform and scheduler use the canonical HTTPS origin and Secure cookies.
- [x] Health checks cover Gateway identity, malformed login, private cache controls, canonical URLs and negative cross-routing.
- [x] Protocol probes execute from the NAS host network namespace rather than the containerized runner loopback.
- [x] Cloudflare/DNS/Synology changes and rollback are documented without secrets.
- [ ] PR #396 exact head passes every applicable workflow.
- [ ] The merged exact image is verified by Character Bazaar Staging Control with sanitized `STAGING_PROVEN` evidence.
- [x] `PRODUCTION_PROVEN` remains false until Issue #91 is completed.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/SynologyStagingNetworkBoundaryTest.php
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
  - merged PR #388
  - merged PR #392
  - Character Bazaar Staging Control
  - Issue #91 production go-live gate
blockers:
  - none for repository validation and bounded staging retry
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T10:04:00Z
session_id: chatgpt-20260801-public-domain-repair-004
policy_version: 2
phase: staging_health_boundary_repair
execution_mode: chat-github-connector
repository_mutation_authorization: PROVEN
external_mutation_scope_authorization: PROVEN
external_operator_access: UNKNOWN
staging_deployment_authorization: PROVEN
context_pressure: medium
decomposition_decision: continue
branch: fix/OTERYN-20260801-host-network-health-check
head: 886c3e7627add514514800ac049dfbe19e2fe386
pr: 396
status: validating
context_routes:
  - agent-governance
  - security
  - auth-identity
  - api
  - testing
owned_paths:
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/SynologyStagingNetworkBoundaryTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
proven:
  - PR #388 merged as 82abef518f91d72d392db4420bb335773087c3e1 after all required exact-head workflows passed.
  - PR #392 merged as b249e5e9cb864ba01376efb273be323b90bcd500 after all path-applicable exact-head workflows passed.
  - Image publication run 30693873144 number 1576 passed for the first merge.
  - Staging run 30693873142 number 5 failed before Docker because partial Marketplace state was treated as a complete deployment environment.
  - PR #392 corrected that loader boundary.
  - Staging run 30694481769 number 6 checked out exact b249e5e9cb864ba01376efb273be323b90bcd500 and resolved both exact images.
  - Run 30694481769 rendered the complete environment, recreated Platform, internal proxy and Gateway, completed migrations and database privilege checks, and verified all expected host bindings.
  - Run 30694481769 then failed when direct Python requests targeted 127.0.0.1 inside the containerized runner namespace rather than the NAS host namespace.
  - Existing public login probing already uses docker run with host networking and proves the correct boundary.
  - PR #396 runs protocol checks in an ephemeral python container with host networking and bounded socket retries.
  - The deterministic repair passed git diff check and Bash syntax validation before direct connector application.
  - No production, Cloudflare, DNS, Canary-source, OTClient or PR #387 evidence mutation occurred.
derived:
  - The second failure is a health-check network-namespace defect, not a Gateway image, binding, migration or credential failure.
  - The partially updated staging runtime must not be labelled STAGING_PROVEN until the full guarded action and evidence upload succeed.
unknown:
  - Exact-head workflow results for PR #396.
  - Result of the next exact staging verification run.
  - Effective Cloudflare certificate, WAF, Access, bot, redirect and HSTS configuration.
  - Exact supported native-client minimum TLS version.
conflicts:
  - The runner is containerized while published loopback ports belong to the NAS host; direct runner-process loopback cannot verify those host bindings.
first_failure:
  marker: runner-loopback-namespace-mismatch
  evidence: run 30694481769 returned ConnectionRefusedError after successful container recreation and binding verification
rejected_hypotheses:
  - Exact Platform and Gateway images were available and resolved.
  - Full environment rendering and the partial-state loader fix succeeded.
  - Platform migration, OAuth client and database privilege checks succeeded.
  - Gateway and Platform host bindings were present and exact.
changed_paths:
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/SynologyStagingNetworkBoundaryTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
validation:
  - command: Character Bazaar Staging Control deploy-enable on b249e5e9cb864ba01376efb273be323b90bcd500
    result: FAIL
    evidence: run 30694481769 number 6 failed at direct runner-loopback Python protocol probe
  - command: deterministic host-network patch validation
    result: PASS
    evidence: git diff check and bash syntax validation passed before connector application
  - command: applicable workflow suite on PR #396 exact head
    result: NOT_RUN
    evidence: GitHub Actions triggered by branch commits
deployment_evidence: Runtime containers were recreated during run 30694481769, but final state persistence and sanitized evidence upload were skipped; STAGING_PROVEN is not established for this repair.
rollback: The deployment process retained the prior image snapshot; no production or public-edge mutation occurred. A later successful exact deployment or explicit guarded rollback must establish final staging state.
blockers:
  - none for exact-head validation and guarded staging retry
next_action: Pass all applicable exact-head workflows on PR #396, squash merge, dispatch Character Bazaar Staging Control for the exact merge SHA, and verify the sanitized STAGING_PROVEN artifact.
```

## Report

`docs/agents/reports/OTERYN-20260801-public-domain-repair.md`

## Notes

Staging completion does not prove the public edge or production. `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until separately authorized external acceptance checks pass.
