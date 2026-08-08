---
task_id: OTERYN-20260801-production-go-live-gate
project_lane: oteryn-platform-core
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
search_first:
  - Issue #91
  - production go-live gate
  - Cloudflare edge repair
optional_reads: []
---

# OTERYN-20260801 production go-live gate

## Lifecycle status

`SUPERSEDED HISTORICAL VALIDATION CHECKPOINT — DO NOT RESUME AS CURRENT PRODUCTION INSTRUCTION`

This task preserves a valid read-only **2026-08-01 staging/public-edge observation generation**. It is no longer the current execution checkpoint for production verification because later evidence changed material public-edge facts and the original next action was already executed by a separate guarded Cloudflare programme.

The durable production authority remains Issue #91. `PRODUCTION_PROVEN=false` remains correct until a future explicitly authorized verification proves every mandatory production criterion against one exact deployed production release.

## Historical result

At the August 1 observation generation:

- the exact observed Synology Compose target was `oteryn-staging` / `STAGING_TARGET`;
- Platform/Gateway source SHA observed was `3eb109b505f7d1c8718cffb823de6d9d5166717c`;
- immutable Canary digest observed was `sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f`;
- all six expected staging containers were running, with healthy bounded Platform/Gateway/MariaDB/Redis probes;
- production configuration verification failed as expected for that staging target;
- no production restore, mutation or smoke was performed;
- the public-edge observation at that time returned WWW Cloudflare 403, Gateway TLS failure, no HTTP-to-HTTPS redirect and HSTS `max-age=0`.

Those statements are historical direct observations tied to the exact August 1 evidence generation. They must not be promoted to claims about the current deployment or current edge configuration.

## Later evidence that supersedes the old edge blockers

Protected-main PR #516 is later terminal evidence for the separately authorized Cloudflare edge programme. It records:

- guarded HSTS apply reaching `state=staged`, `max_age=2592000`;
- complete public E2E PASS with `positive_hsts_www=true`;
- independent trusted-main audit reproducing the staged HSTS target with `mutation=none`;
- stable WAF/Bot repair, canonical skip-rule ordering and Bot Fight Mode disabled;
- Cloudflare task archived and ownership released.

Therefore the August 1 WWW-403/HSTS-zero and “obtain Cloudflare zone-edge audit/apply authorization” statements are **not current blockers or next action**.

Open PR #541 separately reconciles a later public-domain checkpoint and also treats edge/HSTS repair as complete, but because #541 is still unmerged it remains corroborating work-in-progress evidence rather than protected-main authority.

Issue #877 owns a different active Cloudflare verification-evidence reconciliation and must not be duplicated here.

## What remains current

Only the high-level production-gate conclusion survives this historical task:

```text
PRODUCTION_PROVEN=false
```

Issue #91 still requires direct exact-release production evidence. This historical task does **not** prove current state for:

- deployed production Platform/Gateway/game revision;
- current Synology runtime/topology;
- production DB/Redis/cache/session/queue configuration;
- backup policy and dated restore evidence;
- rollback mechanism;
- production mail delivery;
- centralized observability/on-call readiness;
- launch-scope and controlled production smoke;
- current cloudflared host/network path;
- any current public endpoint behavior not backed by a later current evidence package.

A future production verification must discover those facts from current `main` and current environment evidence rather than inheriting August 1 UNKNOWN/blocker lists.

## Current next action

There is **no privileged Cloudflare apply next action in this historical task**.

If the repository owner authorizes a fresh production go-live attempt, start a new current-main bounded Issue #91 verification task that:

1. pins the exact release under evaluation;
2. reads current canonical production-readiness and public-edge authority;
3. performs read-only discovery first;
4. separates repository, staging and production evidence;
5. requests any mutation only after an exact current failure proves it necessary, with explicit rollback and owner authorization;
6. never reuses this historical branch as the production source of truth.

## Safety boundary

This reconciliation authorizes no:

- Cloudflare mutation or token creation;
- Synology deployment/restart/configuration change;
- Environment/secret/variable modification;
- database/Redis/game mutation;
- backup/restore action;
- production smoke or user-data mutation;
- external-repository write.

## PR #405 disposition

PR #405 must not be merged into current `main`. Its unique sanitized August 1 evidence remains recoverable from the closed PR/branch and Git history.

After Issue #885 updates this historical checkpoint/evidence, PR #405 should be closed unmerged as superseded. Issue #91 remains open and authoritative for any future production go-live proof.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
task_kind: validation
implementation_authorized: false
production_mutation_authorized: false
phase: lifecycle-reconciliation
session_id: github-20260808-issue885
session_role: architecture-governance-repair
execution_mode: github_only
updated_at: 2026-08-08T10:22:00+02:00
head: pending-validation-commit
branch: repair/issue-885
pr: none
status: blocked
context_routes:
  - agent-governance
  - testing
  - security
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-production-go-live-gate.md
  - docs/agents/evidence/OTERYN-20260801-production-go-live-gate/index.md
proven:
  - The August 1 evidence directly observed a healthy staging target and a then-broken public edge; it did not prove production.
  - Protected-main PR 516 later superseded the old edge/HSTS blocker generation and archived the guarded Cloudflare edge programme.
  - Issue 91 remains open and PRODUCTION_PROVEN remains false.
  - PR 405 remains an old unmerged branch and must not direct a fresh production attempt.
derived:
  - Closing PR 405 unmerged preserves historical evidence while preventing stale privileged Cloudflare instructions from being resumed.
unknown:
  - Current production deployment and every Issue 91 criterion not directly proven by a fresh exact-release production verification remain unknown.
conflicts: []
first_failure:
  marker: stale-lifecycle-evidence
  evidence: the historical checkpoint still presented superseded public-edge failures and a completed Cloudflare authorization path as current blockers/next action
rejected_hypotheses:
  - the August 1 WWW 403 and HSTS max-age=0 observations remain current merely because PR 405 is still open
  - merging PR 405 is necessary to preserve its sanitized evidence
  - PR 516 proves the wider Issue 91 production gate
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-production-go-live-gate.md
  - docs/agents/evidence/OTERYN-20260801-production-go-live-gate/index.md
validation:
  - command: historical-vs-later edge evidence reconciliation
    result: PASS
    evidence: PR 516 supersedes the old edge/HSTS blocker generation while explicitly preserving PRODUCTION_PROVEN=false
  - command: production verification
    result: BLOCKED
    evidence: no fresh exact-release production verification is authorized or performed by Issue 885
  - command: exact-head Agent Governance
    result: NOT_RUN
    evidence: required after the coherent reconciliation commit
blockers:
  - fresh production verification remains separately owner-gated under Issue 91
next_action: Reconcile the historical evidence index, validate the two-file stacked documentation repair, then close PR 405 as superseded without merging it to current main.
```
