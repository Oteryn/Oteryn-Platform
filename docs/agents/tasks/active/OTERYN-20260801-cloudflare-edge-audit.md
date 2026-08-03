---
task_id: OTERYN-20260801-cloudflare-edge-audit
project_lane: oteryn-platform-core
status: blocked
branch: docs/OTERYN-20260803-hsts-zone-settings-blocker
base_branch: main
updated: 2026-08-03T19:58:00+02:00
feature_pr: pending
---

# OTERYN-20260801 Cloudflare public-edge repair

## Goal

Complete the canonical public edge for:

```text
oteryn.molehill.cloud
gateway.molehill.cloud
```

while preserving the unrelated country restriction, then enable a conservative reversible HSTS stage.

## Current verdict

WAF, Bot, DNS, TLS, certificates, redirects, WWW and Gateway acceptance are complete and stable:

```text
repair_rule_count=1
repair_state=current
bot_fight_mode=false
desired_state=true
public_verdict=PASS
failed_required_checks=none
```

The public E2E passed twice. The second run was idempotent and reported `mutation=none`.

The HSTS stage-1 implementation is merged and its trusted read-only preflight proved the exact baseline:

```yaml
enabled: true
max_age: 0
include_subdomains: true
preload: true
nosniff: true
mutation: none
```

Both operational markers are reset to `inert`. Live HSTS apply is externally blocked only because the existing `Oteryn Edge Audit` token still has `Zone Settings → Read`; the API requires zone-bounded `Zone Settings → Edit` for `PATCH /zones/{zone_id}/settings/security_header`.

## Acceptance criteria

- [x] Gateway hostname, Tunnel, DNS and Universal SSL are current.
- [x] Exact canonical-host WAF skip is first and the unrelated country rule remains unchanged.
- [x] Bot Fight Mode is disabled and independently verified.
- [x] Public WWW and Gateway E2E passed twice with no failed required checks.
- [x] Public-edge repair is idempotent and independently audited.
- [x] HSTS stage-1 audit/apply/rollback implementation passes exact-head validation and is merged.
- [x] Trusted HSTS preflight confirms the exact `max-age=0` baseline.
- [x] Repair and HSTS markers are reset to inert while blocked.
- [ ] Existing token receives only zone-bounded `Zone Settings → Edit` for `molehill.cloud`.
- [ ] Trusted HSTS apply reaches the exact one-month target.
- [ ] Public E2E remains PASS and observes positive HSTS.
- [ ] Markers are reset to inert after terminal validation.
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
  - CLOUDFLARE_EDGE_AUDIT_TOKEN
blockers:
  - Existing token requires Zone Settings Edit for the specific zone molehill.cloud before HSTS apply.
cross_repository_tasks:
  - native-client endpoint rollout remains separately controlled
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-03T19:58:00+02:00
head: 98ab628d7e2c4cccd309db69c50418e22cacbe08
branch: docs/OTERYN-20260803-hsts-zone-settings-blocker
pr: pending
status: blocked
phase: external_zone_settings_write
session_id: chat-20260803-cloudflare-edge-repair
session_role: implementer
execution_mode: chat-github
execution_reason: all repository and read-only production work is complete; exact external token write scope is required
run_scope: bounded_task
continuation_policy: continue_until_real_stop
task_completion_policy: complete_merge_archive
context_routes:
  - agent-governance
  - security
  - api
  - testing
context_pressure: low
context_growth: stable
context_score: 3
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: external permission correction must precede the marker-only HSTS apply
validation_level: live_protected_environment
last_completed_step: prove the exact HSTS baseline and reset both operational markers to inert
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
  - ops/triggers/cloudflare-oteryn-hsts-stage1.md
proven:
  - Reorder apply run 30836740158 reached desired WAF/Bot state and public_verdict PASS.
  - Independent repair audit run 30837198173 reproduced desired_state true and mutation none.
  - Idempotent public run 30837673447 returned mutation none and a second public_verdict PASS.
  - HSTS implementation PR 506 merged as 7704f1c60a2be358bf4d709065313e40d80e7856 after ten exact-head workflows passed.
  - Trusted HSTS audit run 30838787219 reported state baseline, max_age zero, include_subdomains true, preload true, nosniff true and mutation none.
  - Official Cloudflare API documentation requires Zone Settings Write for editing security_header.
derived:
  - Public edge reachability and WAF/Bot behavior are no longer blockers.
  - A one-month target without includeSubDomains or preload limits browser lock-in risk.
unknown:
  - Whether Zone Settings Edit will be effective until the owner saves the token policy and a protected apply is attempted.
conflicts: []
first_failure:
  marker: hsts-zone-settings-write-unavailable
  evidence: current token policy screenshot shows Zone Settings Read while PATCH requires Zone Settings Write/Edit
rejected_hypotheses:
  - HSTS should be enabled before stable public acceptance; two independent public passes now exist.
  - A 12-month preloaded policy is appropriate as the first stage; the accepted target is one month without inheritance or preload.
  - Another WAF/Bot change is required; current desired state and public acceptance are stable.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
  - ops/triggers/cloudflare-oteryn-hsts-stage1.md
validation:
  - command: trusted reorder apply/public run 30836740158
    result: PASS
    evidence: desired state and complete public acceptance
  - command: trusted independent repair audit run 30837198173
    result: PASS
    evidence: desired state remained stable with mutation none
  - command: trusted idempotent public run 30837673447
    result: PASS
    evidence: second public acceptance and mutation none
  - command: trusted HSTS baseline audit run 30838787219
    result: PASS
    evidence: exact max-age-zero baseline and mutation none
blockers:
  - Change only Zone Settings from Read to Edit on Oteryn Edge Audit for specific zone molehill.cloud.
next_action: After the owner saves Zone Settings Edit, run one marker-only HSTS apply, verify public PASS with positive HSTS, reset markers inert, archive the task and release ownership.
```
