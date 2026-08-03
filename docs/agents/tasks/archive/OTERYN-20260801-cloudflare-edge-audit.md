---
task_id: OTERYN-20260801-cloudflare-edge-audit
project_lane: oteryn-platform-core
status: completed
branch: chore/OTERYN-20260804-cloudflare-edge-closeout
base_branch: main
updated: 2026-08-04T00:05:00+02:00
feature_pr: closeout
---

# OTERYN-20260801 Cloudflare public-edge repair

## Goal

Complete the canonical public edge for:

```text
oteryn.molehill.cloud
gateway.molehill.cloud
```

while preserving the unrelated broad country restriction, then promote HSTS through a conservative reversible first stage.

## Terminal verdict

```text
repair_rule_count=1
repair_state=current
repair_before_candidate=true
bot_fight_mode=false
desired_state=true
public_verdict=PASS
failed_required_checks=none
hsts_state=staged
hsts_max_age=2592000
hsts_include_subdomains=false
hsts_preload=false
positive_hsts_www=true
```

The exact canonical-host skip remains first, the unrelated country restriction remains unchanged, Bot Fight Mode is disabled, public WWW and Gateway acceptance pass, and HSTS is staged for one month without `includeSubDomains` or preload.

`PUBLIC_DOMAIN_LAUNCH_READY=false` and `PRODUCTION_PROVEN=false` intentionally remain unchanged. Wider production go-live verification is separate work tracked by Issue #91 and its production-gate record; it is not part of this bounded Cloudflare edge task.

## Acceptance criteria

- [x] Gateway hostname, Tunnel, DNS and Universal SSL are current.
- [x] Exact canonical-host WAF skip is first and the unrelated country rule remains unchanged.
- [x] Bot Fight Mode is disabled and independently verified.
- [x] Public WWW and Gateway E2E passed twice with no failed required checks.
- [x] Public-edge repair is idempotent and independently audited.
- [x] HSTS audit/apply/rollback implementation passes exact-head validation and is merged.
- [x] Trusted HSTS preflight confirmed the exact `max-age=0` baseline.
- [x] Zone-bounded `Zone Settings → Edit` was proven effective by the guarded transition.
- [x] Trusted HSTS apply reached the exact one-month target.
- [x] Public E2E remained `PASS` and observed positive HSTS.
- [x] A final trusted read-only audit reproduced the staged state with `mutation=none`.
- [x] Public-edge and HSTS operational markers are returned to `inert` by closeout.
- [x] Task is archived and ownership is released.

## Terminal evidence

```yaml
proven:
  public_edge_reorder_apply:
    run: 30836740158
    result: PASS
  public_edge_independent_audit:
    run: 30837198173
    result: PASS
  public_edge_idempotent_e2e:
    run: 30837673447
    result: PASS
  public_edge_final_audit:
    run: 30845849631
    result: PASS
  hsts_baseline_audit:
    run: 30838787219
    state: baseline
    max_age: 0
    mutation: none
  dependency_security_repair:
    pr: 512
    merge_commit: 397fab54b44a97eb02f4a2ce82e156d23d1c39a2
    result: PASS
  hsts_apply:
    pr: 513
    trusted_sha: ee5134d4544f237dab4097a0cbfcaf91e474c8af
    run: 30855934824
    state: staged
    max_age: 2592000
    public_verdict: PASS
    positive_hsts_www: true
  hsts_marker_cleanup:
    pr: 514
    merge_commit: 7178fb6e6646b4396f0ae7f9c7ca1df25c1bca8a
  hsts_final_independent_audit:
    pr: 515
    trusted_sha: fa687ab65b7518096f17b8a74f49410fbf37f410
    run: 30857136575
    state: staged
    desired_state: true
    mutation: none
related_prs:
  - "#511 closed superseded without merge"
  - "#512 merged"
  - "#513 merged"
  - "#514 merged"
  - "#515 merged"
```

## Closeout

```yaml
closeout:
  implementation_complete: true
  vertical_slice_complete: true
  audit:
    result: PASS
    independent_validator: trusted-main read-only HSTS audit 30857136575
    material_findings_open: 0
  e2e:
    result: PASS
    journeys:
      - public-www-dns-tls-http-hsts
      - public-gateway-dns-tls-http-contract
  final_ci:
    operational_head: fa687ab65b7518096f17b8a74f49410fbf37f410
    result: PASS
    required_checks:
      - Cloudflare Oteryn HSTS Stage 1
      - CI
      - Edge Security Emulation
      - Game Auth Ticket Concurrency
      - Platform DB Outage Validation
      - Phase 7 Production-Like Validation
  pull_requests:
    open_operational_prs: 0
    unresolved_review_threads: 0
    terminal_prs:
      - "#511 closed superseded"
      - "#512 merged"
      - "#513 merged"
      - "#514 merged"
      - "#515 merged"
  task_status: completed
  task_archived: true
  ownership_released: true
  stale_branches_reconciled: true
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
status: completed
phase: terminal_closeout
execution_mode: chat-github
run_scope: bounded_task
continuation_policy: continue_until_real_stop
task_completion_policy: complete_merge_archive
last_completed_step: independently verify staged HSTS and return both operational markers to inert
owned_paths: []
blockers: []
unknown:
  - Wider production launch acceptance remains outside this task and does not change PRODUCTION_PROVEN.
next_action: none
```
