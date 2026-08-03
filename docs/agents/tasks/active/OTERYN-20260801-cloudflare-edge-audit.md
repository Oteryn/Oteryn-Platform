---
task_id: OTERYN-20260801-cloudflare-edge-audit
project_lane: oteryn-platform-core
status: implementing
branch: feat/OTERYN-20260803-cloudflare-hsts-stage1
base_branch: main
updated: 2026-08-03T19:42:00+02:00
feature_pr: pending
---

# OTERYN-20260801 Cloudflare public-edge repair

## Goal

Complete the canonical public-edge repair for:

```text
oteryn.molehill.cloud
gateway.molehill.cloud
```

while preserving the broad country restriction for unrelated hosts, then promote HSTS through a conservative reversible first stage.

## Current verdict

The WAF/Bot repair is complete and stable:

```text
repair_rule_count=1
repair_state=current
bot_fight_mode=false
desired_state=true
```

The exact canonical-host skip is first in the custom ruleset. Two complete public E2E observations passed with no failed required checks. The second run was idempotent and reported `mutation=none`.

HSTS remains at `max-age=0`. A separate stage-1 audit/apply/rollback implementation is being added with this target:

```yaml
enabled: true
max_age: 2592000
include_subdomains: false
preload: false
nosniff: true
```

## Acceptance criteria

- [x] Gateway hostname, Tunnel, DNS and Universal SSL are current.
- [x] Zone WAF Edit and Bot Management Edit are effective for `molehill.cloud`.
- [x] One exact canonical-host skip rule is first in the custom ruleset.
- [x] The unrelated broad country restriction remains unchanged.
- [x] Bot Fight Mode is disabled and independently re-read as false.
- [x] Deterministic create/reorder/idempotency/rollback tests pass.
- [x] Trusted reorder apply reached `desired_state=true`.
- [x] Public WWW and Gateway DNS/TLS/HTTP E2E passed twice.
- [x] Independent read-only repair audit reproduced the exact desired state.
- [ ] Stage-1 HSTS audit/apply/rollback implementation passes exact-head validation and merges inert.
- [ ] Read-only HSTS preflight confirms the exact `max-age=0` baseline.
- [ ] Existing token receives zone-bounded `Zone Settings Edit` only after the preflight.
- [ ] Trusted HSTS apply reaches the exact staged target and public E2E remains PASS with positive HSTS.
- [ ] HSTS and repair markers are reset to inert.
- [ ] Task is archived and ownership released.
- [ ] `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until wider production acceptance passes.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-public-edge-repair.yml
  - .github/workflows/cloudflare-oteryn-hsts-stage1.yml
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/archive/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/operations/CLOUDFLARE_PUBLIC_EDGE_REPAIR.md
  - docs/operations/CLOUDFLARE_HSTS_STAGE1.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
  - ops/triggers/cloudflare-oteryn-hsts-stage1.md
  - scripts/operations/cloudflare-oteryn-public-edge-failure.py
  - scripts/operations/cloudflare-oteryn-public-edge-repair.py
  - scripts/operations/cloudflare-oteryn-hsts-stage1.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/**
  - tests/operations/cloudflare-oteryn-hsts-stage1/**
modules:
  - operations
  - edge-security
  - game-gateway
dependencies:
  - production-cloudflare GitHub environment
  - CLOUDFLARE_EDGE_AUDIT_TOKEN with Zone WAF Edit and Bot Management Edit
  - Zone Settings Edit is required only for the later HSTS apply
blockers: []
cross_repository_tasks:
  - native-client endpoint rollout remains separately controlled
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-03T19:42:00+02:00
head: 1c168a0d17a1b5f99763068b8f14c1d4e1825ec6
branch: feat/OTERYN-20260803-cloudflare-hsts-stage1
pr: pending
status: implementing
phase: hsts_stage1_implementation
session_id: chat-20260803-cloudflare-edge-repair
session_role: implementer
execution_mode: chat-github
execution_reason: public edge is stable; reversible HSTS promotion remains before task closeout
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
context_score: 5
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: merge and audit HSTS implementation before requesting the additional write permission
validation_level: live_protected_environment
last_completed_step: prove a second idempotent public E2E PASS and implement staged HSTS audit/apply/rollback
owned_paths:
  - .github/workflows/cloudflare-oteryn-hsts-stage1.yml
  - docs/operations/CLOUDFLARE_HSTS_STAGE1.md
  - ops/triggers/cloudflare-oteryn-hsts-stage1.md
  - scripts/operations/cloudflare-oteryn-hsts-stage1.py
  - tests/operations/cloudflare-oteryn-hsts-stage1/**
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
proven:
  - Reorder apply run 30836740158 moved the exact repair rule first and returned public_verdict PASS with no failed checks.
  - Independent repair audit run 30837198173 reproduced repair_state current, Bot Fight Mode false, desired_state true and mutation none.
  - Idempotent apply/public run 30837673447 returned mutation none and a second public_verdict PASS with no failed checks.
  - HSTS remains enabled with max_age zero; include_subdomains, preload and nosniff are true.
  - Cloudflare requires Zone Settings Write for PATCH /zones/{zone_id}/settings/security_header.
  - Cloudflare warns that HTTPS must remain continuously available during the cached HSTS lifetime.
derived:
  - DNS, TLS, Tunnel, certificate, WAF, Bot and application reachability are no longer blockers for this task.
  - A one-month target without includeSubDomains or preload is safer than immediately enabling a long-lived parent-domain policy.
unknown:
  - Whether Zone Settings Edit is effective for the existing token; it currently has Read.
  - Exact live HSTS baseline immediately before apply until trusted preflight runs.
conflicts: []
first_failure:
  marker: hsts-max-age-disabled
  evidence: both public PASS observations report hsts_max_age zero and positive_hsts_www false
rejected_hypotheses:
  - The public edge remains blocked by WAF or Bot controls; two complete public E2E runs passed.
  - A 12-month preloaded policy should be enabled immediately; staged one-month non-preloaded HSTS limits browser lock-in risk.
  - HSTS can be safely changed without rollback; the API operation has irreversible client-side effects during max-age and therefore requires exact baseline checks.
changed_paths:
  - .github/workflows/cloudflare-oteryn-hsts-stage1.yml
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/operations/CLOUDFLARE_HSTS_STAGE1.md
  - ops/triggers/cloudflare-oteryn-hsts-stage1.md
  - scripts/operations/cloudflare-oteryn-hsts-stage1.py
  - tests/operations/cloudflare-oteryn-hsts-stage1/mock_cloudflare.py
  - tests/operations/cloudflare-oteryn-hsts-stage1/run.sh
validation:
  - command: trusted reorder apply and public validation run 30836740158
    result: PASS
    evidence: exact desired WAF/Bot state and complete public acceptance
  - command: trusted independent repair audit run 30837198173
    result: PASS
    evidence: exact desired state remained stable with mutation none
  - command: trusted idempotent apply and public validation run 30837673447
    result: PASS
    evidence: mutation none and second complete public acceptance
  - command: deterministic HSTS stage-1 tests
    result: NOT_RUN
    evidence: exact-head GitHub Actions will run after PR creation
blockers: []
next_action: Open and validate the HSTS stage-1 implementation PR, merge it inert, run a read-only live baseline audit, then request only Zone Settings Edit before apply.
```
