---
task_id: OTERYN-20260801-cloudflare-edge-audit
project_lane: oteryn-platform-core
status: implementing
branch: fix/OTERYN-20260802-cloudflare-public-edge-repair
base_branch: main
updated: 2026-08-02T12:13:00+02:00
feature_pr: pending
---

# OTERYN-20260801 Cloudflare public-edge repair

## Goal

Restore public WWW and Game Gateway traffic after the completed hostname/Tunnel/DNS/certificate migration. Apply the smallest reversible Cloudflare policy change that preserves the country restriction for unrelated hosts while allowing exactly:

```text
oteryn.molehill.cloud
gateway.molehill.cloud
```

## Delivery classification

```yaml
feature_scope:
  type: infrastructure
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: true
  e2e_required: true
implementation_authorized: true
external_protected_configuration_authorized: true
external_authorization_evidence: owner confirmed the dedicated token now has required Edit permissions and requested continuation
```

## Acceptance criteria

- [x] Canonical Gateway hostname, Tunnel, DNS and Universal SSL certificate are proven current.
- [x] Read-only run `30713497073` proves DNS/TLS PASS but public HTTP FAIL.
- [x] The live zone has one broad enabled custom WAF block candidate, Bot Fight Mode enabled, Browser Integrity Check enabled and Security Level `high`.
- [ ] Add one exact canonical-host skip rule before the broad block rule; skip only remaining custom rules, Browser Integrity Check and Security Level.
- [ ] Disable Bot Fight Mode while preserving every other Bot Management field; Bot Fight Mode cannot be scoped or skipped for arbitrary native clients.
- [ ] Provide exact `audit`, `apply` and `rollback` modes with fail-closed ambiguity checks, post-write verification and sanitized evidence.
- [ ] Pull-request validation receives no production secret and all operational modes execute only from trusted `main` through `production-cloudflare`.
- [ ] Independent public DNS/TLS/HTTP E2E passes after apply, or the operation rolls back/records the exact first remaining blocker.
- [ ] Required exact-head CI, independent audit, review and related-PR hygiene pass.
- [ ] Reset operational markers to inert, archive the terminal task and release ownership after verified completion.
- [ ] Keep `PUBLIC_DOMAIN_LAUNCH_READY=false` and `PRODUCTION_PROVEN=false` unless the wider production gate independently passes.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-public-edge-repair.yml
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/archive/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/operations/CLOUDFLARE_PUBLIC_EDGE_REPAIR.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
  - ops/triggers/oteryn-public-edge-validation.md
  - scripts/operations/cloudflare-oteryn-public-edge-repair.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/**
modules:
  - operations
  - edge-security
  - game-gateway
dependencies:
  - production-cloudflare GitHub environment
  - CLOUDFLARE_EDGE_AUDIT_TOKEN with Zone WAF Edit and Bot Management Edit
  - existing trusted public-edge validator
blockers: []
cross_repository_tasks:
  - native-client endpoint rollout remains separately controlled
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-02T12:13:00+02:00
head: 7189fc48c312f7e7fbab7ce59413aa1a2d666006
branch: fix/OTERYN-20260802-cloudflare-public-edge-repair
pr: pending
status: implementing
phase: implement
session_id: chat-20260802-cloudflare-edge-repair
session_role: implementer
execution_mode: chat-github
execution_reason: GitHub-only multi-file infrastructure change with protected-environment validation
run_scope: bounded_task
continuation_policy: continue_until_real_stop
task_completion_policy: complete_merge_archive
context_routes:
  - agent-governance
  - security
  - api
  - testing
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one cohesive Cloudflare policy repair followed by real public E2E and closeout
validation_level: focused
last_completed_step: claimed repair phase and recorded owner authorization
owned_paths:
  - .github/workflows/cloudflare-oteryn-public-edge-repair.yml
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/archive/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/operations/CLOUDFLARE_PUBLIC_EDGE_REPAIR.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
  - ops/triggers/oteryn-public-edge-validation.md
  - scripts/operations/cloudflare-oteryn-public-edge-repair.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/**
proven:
  - Main at task claim is f4ffe15a0419279894e11e2ebc23d512bd7a6c3d.
  - Run 30713497073 artifact 8822613239 proves DNS and TLS for both canonical hosts.
  - The active universal certificate covers both canonical one-label hostnames and not the retired hostname.
  - Custom ruleset 67ca2e19272a4c7d97c2a53681d0eb2f contains broad block rule e0f91939eb494d4490d975498a9a9724 with sanitized expression hash 3f5a9e27f91d9cfe4fb6f77ede8c1e91997ef32a91a443cd1e6b61211ff13c45.
  - Bot Fight Mode and JavaScript detections are enabled; Browser Integrity Check is on and Security Level is high.
  - Every representative public request in the last observation returned a Cloudflare 403 interstitial before reaching Platform or Gateway.
  - Cloudflare documents that Bot Fight Mode cannot be bypassed by a WAF skip rule; zone-wide disable is required for arbitrary native clients.
derived:
  - A canonical-host skip rule can preserve the broad country block for all unrelated hostnames.
  - Skipping BIC and Security Level only for canonical hosts plus disabling non-scopable Bot Fight Mode is the smallest complete policy repair for both browser and native-client traffic.
unknown:
  - Whether the edited token's write permissions are effective until the protected apply preflight runs.
  - Whether origin/application behavior passes once Cloudflare interstitials are removed.
conflicts: []
first_failure:
  marker: cloudflare-policy-interstitial
  evidence: public edge run 30713497073 returned 403 interstitials for all representative WWW and Gateway requests
rejected_hypotheses:
  - DNS, Tunnel or certificate remains the blocker; all are proven current.
  - A host-scoped WAF skip can bypass Bot Fight Mode; Cloudflare documents that it cannot.
  - Disabling the broad country rule globally is required; an earlier exact-host skip rule preserves it for unrelated hosts.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
validation: []
blockers: []
invocation_started_at: 2026-08-02T12:13:00+02:00
last_progress_at: 2026-08-02T12:13:00+02:00
runtime_limit_minutes: 60
no_progress_minutes: 15
ci_checks_for_current_head: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 1
stall_warnings: 0
next_action: implement the exact audited WAF/Bot repair, rollback logic, focused tests and trusted-main workflow
```
