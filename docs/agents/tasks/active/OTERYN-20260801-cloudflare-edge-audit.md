---
task_id: OTERYN-20260801-cloudflare-edge-audit
project_lane: oteryn-platform-core
status: implementing
branch: fix/OTERYN-20260803-cloudflare-skip-rule-priority
base_branch: main
updated: 2026-08-03T19:10:00+02:00
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

The dedicated zone-bounded token is effective for both WAF and Bot Management writes. Trusted apply run `30834596610` completed the originally intended state:

```text
repair_rule_count=1
repair_state=current
repair_before_candidate=true
bot_fight_mode=false
desired_state=true
mutation=bot_fight_mode_disabled
```

DNS, TLS, certificates and HTTP-to-HTTPS redirects pass. Public HTTPS requests still receive Cloudflare `403` interstitials. Sanitized ruleset evidence proves that the exact canonical skip rule is not first: an earlier enabled skip rule can skip the remaining current ruleset without skipping Browser Integrity Check or Security Level. The later Oteryn rule can therefore be shadowed before its `bic` and `securityLevel` product exemptions execute.

The repair contract is being tightened so the exact-host rule must be the first rule in the zone custom ruleset. The apply path will create it first or move the existing exact rule first, preserve the unrelated country restriction, and restore its previous position if a later operation fails.

## Acceptance criteria

- [x] Gateway hostname, Tunnel, DNS and Universal SSL certificate are current.
- [x] Zone WAF Edit and Bot Management Edit are effective for `molehill.cloud`.
- [x] One exact canonical-host WAF skip rule exists before the audited country block.
- [x] Bot Fight Mode is disabled and independently re-read as false.
- [x] HTTP-to-HTTPS redirects pass for both canonical hosts.
- [x] Public evidence proves the remaining failure is Cloudflare HTTPS interstitial behavior, not DNS, TLS, Tunnel or origin certificate failure.
- [ ] Repair rule priority requires index zero so an earlier current-ruleset skip cannot shadow the product exemptions.
- [ ] Creation, reordering, idempotency and emergency position rollback pass deterministic tests.
- [ ] Trusted-main apply moves the existing exact repair rule first without recreating it.
- [ ] Public WWW and Gateway DNS/TLS/HTTP E2E passes.
- [ ] HSTS is promoted from `max-age=0` only after stable public acceptance.
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
  - CLOUDFLARE_EDGE_AUDIT_TOKEN with Zone WAF Edit and Bot Management Edit for molehill.cloud
blockers: []
cross_repository_tasks:
  - native-client endpoint rollout remains separately controlled
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-03T19:10:00+02:00
head: eefbdd35888aa955c679e17947f6d20d70c26b29
branch: fix/OTERYN-20260803-cloudflare-skip-rule-priority
pr: pending
status: implementing
phase: canonical_skip_rule_priority
session_id: chat-20260803-cloudflare-edge-repair
session_role: implementer
execution_mode: chat-github
execution_reason: live policy writes succeeded; public evidence isolated a rule-order shadowing defect
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
decomposition_reason: rule-order implementation must merge before a separate marker-only live reorder
validation_level: live_protected_environment
last_completed_step: reset the operational marker to inert and implement first-position enforcement with position rollback
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
  - Permission preflight run 30832409317 succeeded with mutation none.
  - WAF create run 30832830466 proved Zone WAF write access and created the exact repair rule.
  - Ruleset-response parser fix PR 498 merged as efab1b1598e6571bfdc3842c7d812c8c84801aa8 after all exact-head gates passed.
  - Post-fix audit run 30834139371 proved one exact repair rule before the country block and Bot Fight Mode enabled.
  - Apply run 30834596610 disabled Bot Fight Mode and reached the previous desired state without another WAF POST.
  - Artifact 8864299649 digest sha256:ebaa31677df740dd070c0b14a44e3e9cc6d864e694223f43bcf887d4af48c606 proves DNS, TLS and redirects pass while every tested HTTPS application request returns a Cloudflare interstitial.
  - Sanitized ruleset evidence shows an enabled current-ruleset skip precedes the canonical repair rule and does not include the bic or securityLevel product keys.
  - Cloudflare documents that rules run in order and a current-ruleset skip skips all remaining rules; placing a rule first is supported by PATCH position before an empty rule ID.
derived:
  - The canonical skip can be shadowed before its Browser Integrity Check and Security Level exemptions execute.
  - Moving the exact-host skip to index zero is narrower than disabling Browser Integrity Check or Security Level globally.
  - The unrelated country restriction remains effective for all noncanonical hosts.
unknown:
  - Public HTTPS behavior after the canonical skip is moved to index zero.
  - Whether positive HSTS should be enabled immediately after first PASS or after an additional stability observation.
conflicts: []
first_failure:
  marker: canonical-skip-shadowed-by-earlier-current-ruleset-skip
  evidence: run 30834596610 returned 403 interstitials while sanitized rule order placed another current-ruleset skip before the Oteryn product-exemption rule
rejected_hypotheses:
  - DNS, Tunnel, certificate or HTTP redirects remain broken; all passed in run 30834596610.
  - Bot Fight Mode remains the blocker; the same run proved fight_mode false.
  - The broad country block must be removed; a canonical exact-host rule can remain isolated while unrelated hosts retain the block.
  - Browser Integrity Check and Security Level must be disabled globally; their selective product skips can execute when the canonical rule is first.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - ops/triggers/cloudflare-oteryn-public-edge-repair.md
  - scripts/operations/cloudflare-oteryn-public-edge-repair.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/mock_cloudflare.py
  - tests/operations/cloudflare-oteryn-public-edge-repair/run.sh
validation:
  - command: Cloudflare apply and public validation run 30834596610
    result: FAIL
    evidence: policy mutation succeeded; public HTTPS acceptance still failed with Cloudflare interstitials
  - command: deterministic priority and rollback tests on implementation branch
    result: NOT_RUN
    evidence: exact-head GitHub Actions will run after PR creation
blockers: []
next_action: Open and validate the skip-priority implementation PR, merge with marker inert, audit the live index, then apply one reorder-only transition and rerun public E2E.
```
