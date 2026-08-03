---
task_id: OTERYN-20260801-cloudflare-edge-audit
project_lane: oteryn-platform-core
status: implementing
branch: fix/OTERYN-20260803-cloudflare-ruleset-create-response
base_branch: main
updated: 2026-08-03T18:38:00+02:00
feature_pr: pending
---

# OTERYN-20260801 Cloudflare public-edge repair

## Goal

Restore public WWW and Game Gateway traffic while preserving the broad country restriction for unrelated hosts. Canonical hosts:

```text
oteryn.molehill.cloud
gateway.molehill.cloud
```

## Current verdict

The owner corrected the dedicated token policy to zone-bounded `Zone WAF: Edit` and `Bot Management: Edit` for `molehill.cloud`. Trusted preflight succeeded. The next apply proved WAF write access by creating the exact managed skip rule in the correct position, but the implementation rejected Cloudflare's documented ruleset-shaped create response before reaching the Bot Management update.

Current live state from the automatic post-failure audit:

```text
candidate_count=1
repair_rule_count=1
repair_state=current
repair_before_candidate=true
bot_fight_mode=true
desired_state=false
```

The partial state is bounded: the exact Oteryn host exception exists before the audited country block, the unrelated country restriction remains, and Bot Fight Mode remains unchanged. The operational marker has been reset to `inert` on the repair branch while the response contract and recovery logic are corrected.

## Acceptance criteria

- [x] Gateway hostname, Tunnel, DNS and Universal SSL certificate are current.
- [x] Exact-host WAF skip plus Bot Fight Mode repair is implemented with audit/apply/rollback.
- [x] Fixed audited rule ID/hash, ambiguity denial, exact confirmations, idempotency and partial rollback are tested.
- [x] Token policy is effective for WAF creation; the exact repair rule now exists and is ordered before the audited broad block.
- [x] Official Cloudflare API evidence proves the create-rule response is a ruleset object containing the rules array.
- [ ] Repair implementation accepts and validates the ruleset-shaped create response.
- [ ] Recovery detects and deletes a rule when Cloudflare accepts POST but response validation fails.
- [ ] Existing exact WAF partial state is completed by disabling Bot Fight Mode without recreating the rule.
- [ ] Trusted-main apply reaches exact desired state and public DNS/TLS/HTTP E2E is rerun.
- [ ] Operational marker is reset to inert after terminal validation.
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
  - CLOUDFLARE_EDGE_AUDIT_TOKEN with effective Zone WAF Edit and Bot Management Edit for molehill.cloud
blockers: []
cross_repository_tasks:
  - native-client endpoint rollout remains separately controlled
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-03T18:38:00+02:00
head: cab9096c9cdfb73b304c1678cb2d3c5667fd54f3
branch: fix/OTERYN-20260803-cloudflare-ruleset-create-response
pr: pending
status: implementing
phase: ruleset_create_response_repair
session_id: chat-20260803-cloudflare-edge-repair
session_role: implementer
execution_mode: chat-github
execution_reason: live write authorization is proven; implementation must reconcile the documented create response and bounded partial state
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
decomposition_reason: implementation correction must merge before the existing exact WAF partial state can be completed safely
validation_level: live_protected_environment
last_completed_step: reset the operational marker to inert and implement ruleset-shaped response normalization plus recovery inference
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
  - Owner screenshot confirms Zone WAF Edit and Bot Management Edit for the specific zone molehill.cloud, with no IP filter or TTL.
  - Trusted preflight run 30832409317 found one audited country rule, zero repair rules, Bot Fight Mode enabled and mutation none.
  - Apply run 30832830466 created the exact repair rule before the audited candidate, then failed while validating the create response before Bot Fight Mode was changed.
  - The automatic post-failure audit reported one current repair rule, correct ordering, Bot Fight Mode enabled and desired state false.
  - Cloudflare official API documentation defines the create-rule response as a ruleset object with a rules array.
  - The implementation branch normalizes that response and infers accepted POST state before emergency rollback.
derived:
  - WAF write authorization is no longer a blocker.
  - The remaining live mutation is limited to disabling Bot Fight Mode because the exact WAF rule is already present.
  - Repeating the old apply implementation would be incorrect; response-contract remediation must merge first.
unknown:
  - Whether Bot Management Edit succeeds in the protected environment; the previous apply failed before attempting that operation.
  - Final public WWW and Gateway behavior after Bot Fight Mode is disabled.
conflicts: []
first_failure:
  marker: cloudflare-ruleset-create-response-shape
  evidence: run 30832830466 classified create_waf_skip_rule as unexpected_api_response while the post-failure audit proved the rule was created
rejected_hypotheses:
  - The new token still lacks WAF write access; the exact rule was created successfully.
  - The WAF rule is absent or incorrectly ordered; the post-failure audit proves current exact state before the candidate.
  - The create endpoint returns only the created rule; Cloudflare documents and live behavior show a ruleset object.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
  - scripts/operations/cloudflare-oteryn-public-edge-repair.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/mock_cloudflare.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/run.sh
validation:
  - command: Cloudflare permission/state preflight run 30832409317
    result: PASS
    evidence: exact live state was readable and mutation was none
  - command: Cloudflare partial apply run 30832830466
    result: FAIL
    evidence: exact WAF rule was created; response validation failed before Bot update; post-failure audit captured bounded partial state
  - command: deterministic repair tests on implementation branch
    result: NOT_RUN
    evidence: exact-head GitHub Actions will run after PR creation
blockers: []
next_action: Open and validate the response-contract repair PR, merge with the marker inert, run a trusted audit, then apply only the missing Bot Fight Mode transition and execute public E2E.
```
