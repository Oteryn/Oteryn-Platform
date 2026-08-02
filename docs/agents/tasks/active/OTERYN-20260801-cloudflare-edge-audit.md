---
task_id: OTERYN-20260801-cloudflare-edge-audit
project_lane: oteryn-platform-core
status: validating
branch: fix/OTERYN-20260802-cloudflare-public-edge-repair
base_branch: main
updated: 2026-08-02T12:26:00+02:00
feature_pr: "456"
---

# OTERYN-20260801 Cloudflare public-edge repair

## Goal

Restore public WWW and Game Gateway traffic after the completed hostname, Tunnel, DNS and certificate migration. Preserve the broad country restriction for unrelated hosts while allowing exactly:

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
external_authorization_evidence: owner confirmed the dedicated token has Zone WAF Edit and Bot Management Edit and requested continuation
```

## Acceptance criteria

- [x] Canonical Gateway hostname, Tunnel, DNS and Universal SSL certificate are proven current.
- [x] Read-only run `30713497073` proves DNS/TLS PASS but public HTTP FAIL.
- [x] Implement an exact canonical-host WAF skip rule before the one broad country block candidate.
- [x] Skip only remaining custom rules, Browser Integrity Check and Security Level for the two canonical hosts.
- [x] Disable non-scopable Bot Fight Mode while preserving the remaining Bot Management configuration.
- [x] Implement exact `audit`, `apply` and `rollback` modes with fail-closed ambiguity checks and post-write verification.
- [x] Automatically roll back a newly created WAF rule if Bot mutation or final verification fails.
- [x] Keep pull-request validation secret-free and live operations restricted to trusted `main` and `production-cloudflare`.
- [x] Deterministic mock tests prove audit, confirmation denial, apply, idempotency, rollback, ambiguity rejection, baseline preservation and partial-failure rollback.
- [ ] Required exact-head CI passes on the final implementation head.
- [ ] Trusted-main read-only preflight proves the edited token can access the exact live state.
- [ ] Trusted-main apply and independent public DNS/TLS/HTTP E2E pass, or an exact remaining blocker is recorded.
- [ ] Fresh audit, review and related-PR hygiene pass.
- [ ] Operational markers are reset to inert and the task is archived after verified completion.
- [ ] `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false unless the wider production gate independently passes.

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
updated_at: 2026-08-02T12:26:00+02:00
head: dd472474c549e6b5537c28c1f49f84863997bcc4
branch: fix/OTERYN-20260802-cloudflare-public-edge-repair
pr: 456
status: validating
phase: validate
session_id: chat-20260802-cloudflare-edge-repair
session_role: implementer
execution_mode: chat-github
execution_reason: GitHub-only infrastructure implementation with protected-environment operation after merge
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
decomposition_reason: one cohesive policy repair followed by real public E2E and closeout
validation_level: exact_head
last_completed_step: deterministic repair and rollback workflow passed focused GitHub Actions validation
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
  - Main at task claim was f4ffe15a0419279894e11e2ebc23d512bd7a6c3d.
  - Run 30713497073 artifact 8822613239 proves DNS and TLS for both canonical hosts while all representative HTTP requests were stopped by Cloudflare interstitials.
  - One enabled broad country block candidate exists and has no hostname predicate.
  - Bot Fight Mode is enabled and cannot be bypassed with a WAF skip rule.
  - The implementation creates one exact-host skip rule before the candidate, disables only fight_mode, verifies the result and supports exact rollback.
  - Focused workflow run 30743867281 job 91485965536 passed on implementation head dd472474c549e6b5537c28c1f49f84863997bcc4.
derived:
  - The exact-host skip preserves the broad country restriction for unrelated services.
  - Disabling Bot Fight Mode is required for arbitrary native-client traffic because it is not host-scopable.
unknown:
  - Whether the edited token write capabilities are effective until the trusted-main preflight runs.
  - Whether origin/application behavior passes after the Cloudflare interstitial controls are removed.
conflicts: []
first_failure:
  marker: cloudflare-policy-interstitial
  evidence: public edge run 30713497073 returned Cloudflare 403 interstitials for every representative WWW and Gateway request
rejected_hypotheses:
  - DNS, Tunnel or certificate remains the blocker; all are proven current.
  - A host-scoped WAF skip can bypass Bot Fight Mode; Cloudflare states that it cannot.
  - The country rule must be disabled globally; an earlier exact-host skip preserves it for unrelated hosts.
changed_paths:
  - .github/workflows/cloudflare-oteryn-public-edge-repair.yml
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/operations/CLOUDFLARE_PUBLIC_EDGE_REPAIR.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
  - scripts/operations/cloudflare-oteryn-public-edge-repair.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/mock_cloudflare.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/run.sh
validation:
  - command: Cloudflare Oteryn Public Edge Repair run 30743867281 / job 91485965536
    result: PASS
    evidence: audit GET-only, exact confirmations, apply, idempotency, rollback, ambiguity denial, baseline-off preservation, partial-failure rollback and workflow trust boundary all passed
blockers: []
invocation_started_at: 2026-08-02T12:13:00+02:00
last_progress_at: 2026-08-02T12:26:00+02:00
runtime_limit_minutes: 60
no_progress_minutes: 15
ci_checks_for_current_head: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 1
stall_warnings: 0
next_action: verify exact-head required CI for the checkpoint commit, perform a fresh audit, then merge PR 456 and run a marker-only trusted-main audit
```
