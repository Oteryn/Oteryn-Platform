---
task_id: OTERYN-20260801-cloudflare-edge-audit
project_lane: oteryn-platform-core
status: blocked
branch: docs/OTERYN-20260802-cloudflare-waf-token-blocker
base_branch: main
updated: 2026-08-02T13:30:00+02:00
feature_pr: pending
---

# OTERYN-20260801 Cloudflare public-edge repair

## Goal

Restore public WWW and Game Gateway traffic while preserving the broad country restriction for unrelated hosts. The canonical hosts remain:

```text
oteryn.molehill.cloud
gateway.molehill.cloud
```

## Current verdict

Repository implementation is complete and validated. Live mutation is externally blocked because the dedicated GitHub secret `CLOUDFLARE_EDGE_AUDIT_TOKEN` can read the zone WAF but Cloudflare rejects creation of a zone custom rule.

```text
failure_phase=create_waf_skip_rule
http_status=403
error_codes=10000
repair_rule_count=0
bot_fight_mode=true
mutation=none
```

The operational marker is reset to `inert`. No managed repair rule exists and Bot Fight Mode remains unchanged.

## Acceptance criteria

- [x] Gateway hostname, Tunnel, DNS and Universal SSL certificate are current.
- [x] Exact-host WAF skip plus Bot Fight Mode repair is implemented with audit/apply/rollback.
- [x] Fixed audited rule ID/hash, ambiguity denial, exact confirmations, idempotency and partial rollback are tested.
- [x] Live read-only preflight proves one broad country candidate, no managed repair rule and Bot Fight Mode enabled.
- [x] Two bounded apply attempts failed at the same WAF rule creation boundary and left no mutation.
- [x] Safe failure diagnostics publish phase, HTTP status, Cloudflare codes and post-failure state without raw API data.
- [x] Operational marker is reset to inert after the bounded retry budget.
- [ ] Existing token policy is corrected so `POST /zones/{zone_id}/rulesets/{ruleset_id}/rules` succeeds with zone-bounded WAF write access.
- [ ] Trusted-main apply reaches exact desired state and public DNS/TLS/HTTP E2E is rerun.
- [ ] Task is archived only after verified completion or an explicit terminal owner decision.
- [ ] `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until wider production acceptance passes.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-public-edge-repair.yml
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/archive/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/operations/CLOUDFLARE_PUBLIC_EDGE_REPAIR.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
  - scripts/operations/cloudflare-oteryn-public-edge-failure.py
  - scripts/operations/cloudflare-oteryn-public-edge-repair.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/**
modules:
  - operations
  - edge-security
  - game-gateway
dependencies:
  - production-cloudflare GitHub environment
  - CLOUDFLARE_EDGE_AUDIT_TOKEN with effective Zone WAF Write/Edit and Bot Management Write/Edit for molehill.cloud
blockers:
  - Cloudflare returns HTTP 403 code 10000 when the dedicated token creates a zone custom WAF rule.
cross_repository_tasks:
  - native-client endpoint rollout remains separately controlled
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-02T13:30:00+02:00
head: be468ccc4f74e1c8f55c0a449f3100fa1a8d5241
branch: docs/OTERYN-20260802-cloudflare-waf-token-blocker
pr: pending
status: blocked
phase: external_dependency
session_id: chat-20260802-cloudflare-edge-repair
session_role: implementer
execution_mode: chat-github
execution_reason: repository implementation is complete; the external Cloudflare token policy rejects the exact authorized write
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
context_score: 4
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: external permission correction must precede another live apply
validation_level: live_protected_environment
last_completed_step: reset the operational marker to inert and record the exact Cloudflare WAF write denial
owned_paths:
  - .github/workflows/cloudflare-oteryn-public-edge-repair.yml
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/archive/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/operations/CLOUDFLARE_PUBLIC_EDGE_REPAIR.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
  - scripts/operations/cloudflare-oteryn-public-edge-failure.py
  - scripts/operations/cloudflare-oteryn-public-edge-repair.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/**
proven:
  - PR 456 merged the reversible repair implementation after all exact-head gates passed.
  - Trusted preflight run 30744856911 found exactly one audited country rule, zero managed repair rules and Bot Fight Mode enabled.
  - Apply run 30744995272 failed and post-failure audit 30745139637 proved zero remaining repair rules, Bot Fight Mode enabled and mutation none.
  - Diagnostic retry run 30745738371 failed at create_waf_skip_rule with HTTP 403 code 10000.
  - The diagnostic retry post-failure state again reported repair_rule_count zero, Bot Fight Mode true and mutation none.
  - Official Cloudflare WAF documentation requires Zone WAF Write for adding a rule to a zone entry-point ruleset.
derived:
  - DNS, Tunnel, certificate and repository implementation are not the current blocker.
  - The effective token policy used by GitHub lacks accepted write authorization for the exact zone Rulesets operation, even though zone WAF reads succeed.
unknown:
  - Whether the Cloudflare UI change was saved on the same token value stored in GitHub.
  - Whether the token resource selector still targets exactly molehill.cloud after editing.
  - Whether Bot Management write is effective; WAF creation fails first, before that operation is attempted.
conflicts:
  - The owner reported adding Edit permissions, but the protected API operation still returns authorization code 10000.
first_failure:
  marker: cloudflare-zone-waf-write-denied
  evidence: run 30745738371 returned failure_phase=create_waf_skip_rule, HTTP 403, code 10000
rejected_hypotheses:
  - The first failure was an unknown payload problem; the diagnostic retry classified it as Cloudflare authorization failure.
  - The apply left a partial rule or disabled Bot Fight Mode; two post-failure audits prove the original state remains.
  - Repeating the same apply is useful; the bounded identical-failure retry budget is exhausted.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
validation:
  - command: Cloudflare public-edge preflight run 30744856911
    result: PASS
    evidence: exact live state was readable and mutation was none
  - command: Cloudflare public-edge apply run 30744995272
    result: FAIL
    evidence: operation failed; follow-up audit proved no mutation
  - command: Cloudflare post-failure audit run 30745139637
    result: PASS
    evidence: repair rule absent, Bot Fight Mode true, mutation none
  - command: Cloudflare diagnostic apply run 30745738371
    result: FAIL
    evidence: create_waf_skip_rule returned HTTP 403 code 10000; post-failure state remained unchanged
blockers:
  - Existing CLOUDFLARE_EDGE_AUDIT_TOKEN must have effective Zone WAF Write/Edit for the specific zone molehill.cloud; the current token is rejected by the create-rule endpoint.
next_action: In Cloudflare edit the same user API token stored as CLOUDFLARE_EDGE_AUDIT_TOKEN, verify Zone WAF is Edit/Write and the resource is Include Specific zone molehill.cloud, save the token policy, then rerun one read-only preflight before any apply.
```
