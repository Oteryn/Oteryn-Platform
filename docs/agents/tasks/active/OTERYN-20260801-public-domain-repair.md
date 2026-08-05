---
task_id: OTERYN-20260801-public-domain-repair
required_reads:
  - AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
search_first:
  - PRs #388, #392 and #396
  - Cloudflare closeout PR #516
  - public-edge PASS runs 30836740158 and 30837673447
  - HSTS apply run 30855934824
  - HSTS final audit run 30857136575
  - Issue #91
---

# OTERYN-20260801-public-domain-repair

## Goal

Repair and prove the canonical public WWW and Game Gateway domain path without weakening application or Cloudflare security boundaries, and obtain controlled end-to-end password-recovery delivery evidence before terminal closeout.

## Acceptance criteria

- [x] Repository canonical URL, Secure-cookie and bounded Gateway checks are merged.
- [x] Exact source `3eb109b505f7d1c8718cffb823de6d9d5166717c` is deployed and `STAGING_PROVEN`.
- [x] Canonical Tunnel ingress and proxied DNS records are reconciled.
- [x] Game Gateway uses the single-level canonical hostname covered by the active certificate.
- [x] Exact canonical WAF skip rule is first and Bot Fight Mode is disabled without weakening unrelated hostname restrictions.
- [x] Public WWW, Gateway, cross-route and HTTP-to-HTTPS acceptance pass.
- [x] HSTS stage 1 is active with `max-age=2592000`, `includeSubDomains=false`, `preload=false` and `nosniff=true`.
- [x] Independent trusted-main audits reproduce the desired Cloudflare state without mutation.
- [ ] Controlled redacted password-recovery delivery passes through the staging mail path with owner-observed mailbox evidence.
- [x] `PRODUCTION_PROVEN` remains false until Issue #91 completes.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
modules:
  - public-web
  - identity
  - game-gateway
  - edge-transport
dependencies:
  - Issue #91 production go-live gate
  - owner-observed staging mailbox
blockers:
  - reset request must be submitted through the public staging form and the owner must report sanitized receipt/completion evidence
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T12:59:00Z
status: waiting
phase: owner_observed_staging_password_recovery
branch: docs/OTERYN-20260805-public-domain-repair-reconcile
head: 48d7d794928541c10dd17a3365235e8b37c059af
pr: 541
context_routes:
  - security
  - auth-identity
  - operations
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
repository_mutation_authorization: PROVEN
external_edge_mutation_authorization: COMPLETED_WITH_EVIDENCE
production_mutation_authorization: NOT_PROVEN
staging_password_reset_test_authorization: PROVEN
proven:
  - PR #516 archived the completed Cloudflare edge task after guarded repair and independent verification.
  - Public-edge runs 30836740158 and 30837673447 passed all required DNS, TLS, WWW, Gateway, cross-route and redirect checks.
  - HSTS apply run 30855934824 reached the staged target and complete public E2E PASS with positive HSTS.
  - HSTS audit run 30857136575 reproduced the exact target with mutation=none.
  - Password recovery implementation is merged and refuses the log mail transport.
  - The repository owner authorized a staging password-reset request for the existing identity associated with a privately supplied mailbox and will manually observe the mailbox.
  - Issue #91 remains open; production mail delivery remains separately ENV-EVIDENCE-REQUIRED.
derived:
  - Public-domain routing and Cloudflare edge repair are complete.
  - Manual owner observation is sufficient for staging delivery evidence when the receipt metadata is sanitized and no reset token, full link or password is persisted.
  - This test may establish STAGING_PROVEN only; it cannot establish PRODUCTION_PROVEN.
unknown:
  - whether the staging runtime currently has a delivery-capable mail transport configured
  - whether the reset message reaches the owner-observed mailbox
  - whether the received link uses the canonical staging host and completes successfully
  - whether token replay is rejected and prior sessions are revoked in the deployed staging runtime
conflicts:
  - the previous checkpoint required automated mailbox access, but the owner has explicitly accepted manual observation for the staging-only proof.
first_failure:
  marker: staging-password-recovery-not-yet-executed
  evidence: no sanitized receipt/completion observation has yet been recorded for the authorized staging identity.
rejected_hypotheses:
  - Cloudflare edge remains broken; final WAF/Bot, public E2E and HSTS evidence passed.
  - A generic forgot-password HTTP 200 proves delivery; it proves only enumeration-safe request handling.
  - Automated Gmail access is required; manual owner observation is adequate for staging evidence.
validation:
  - command: public-edge repair apply and E2E
    result: PASS
    evidence: runs 30836740158 and 30837673447
  - command: guarded HSTS stage-1 apply
    result: PASS
    evidence: run 30855934824
  - command: independent final HSTS audit
    result: PASS
    evidence: run 30857136575
  - command: controlled staging password-recovery delivery
    result: NOT_RUN
    evidence: awaiting public form submission and sanitized owner mailbox observation
blockers:
  - owner must submit the authorized mailbox through the public staging forgot-password form because the current agent tools cannot safely fill and submit a CSRF-protected browser form without exposing the address in public GitHub automation
next_action: Owner submits the authorized staging mailbox through the public forgot-password form, then reports only receipt time, sender, subject and link hostname; do not paste the full reset URL, token or password. After reset completion, record whether login succeeds, replay fails and prior sessions are revoked, then archive this task as STAGING_PROVEN while leaving PRODUCTION_PROVEN=false.
```

## Current classification

```text
STAGING_PROVEN=true
PUBLIC_EDGE_REPAIR_COMPLETE=true
PUBLIC_EDGE_E2E_PASS=true
HSTS_STAGE1_COMPLETE=true
PASSWORD_RECOVERY_DELIVERY_PROVEN=false
PUBLIC_DOMAIN_TASK_CLOSED=false
PUBLIC_DOMAIN_LAUNCH_READY=false
PRODUCTION_PROVEN=false
```
