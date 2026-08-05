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
- [ ] Controlled redacted password-recovery delivery passes through a real delivery-capable mail path and controlled mailbox.
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
  - controlled production or approved non-production mail target
blockers:
  - no controlled test identity/mailbox and delivery-capable target are provisioned for a real password-reset delivery proof
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T12:45:00Z
status: blocked
phase: password_recovery_delivery_evidence
branch: docs/OTERYN-20260805-public-domain-repair-reconcile
head: pending
pr: pending
context_routes:
  - security
  - auth-identity
  - operations
  - testing
repository_mutation_authorization: PROVEN
external_edge_mutation_authorization: COMPLETED_WITH_EVIDENCE
production_mutation_authorization: NOT_PROVEN
proven:
  - PR #516 archived the completed Cloudflare edge task after guarded repair and independent verification.
  - Public-edge runs 30836740158 and 30837673447 passed all required DNS, TLS, WWW, Gateway, cross-route and redirect checks.
  - HSTS apply run 30855934824 reached the staged target and complete public E2E PASS with positive HSTS.
  - HSTS audit run 30857136575 reproduced the exact target with mutation=none.
  - Password recovery implementation is merged and refuses the log mail transport.
  - Issue #91 remains open and production mail delivery is explicitly ENV-EVIDENCE-REQUIRED.
derived:
  - Public-domain routing and Cloudflare edge repair are complete.
  - Password-recovery delivery cannot be proven by an anonymous HTTP probe or an array/log mailer.
  - A real test requires a controlled existing identity, delivery-capable mail configuration and mailbox observation without exposing credentials or reset tokens.
unknown:
  - effective real mail provider and sender-domain readiness for the intended target
  - controlled test identity and mailbox available to this task
  - delivery, receipt and reset-link completion result
conflicts:
  - the previous checkpoint reported a Cloudflare token blocker, but later merged work and live evidence removed that blocker.
first_failure:
  marker: controlled-password-recovery-target-unavailable
  evidence: repository search found no trusted-main password-recovery delivery workflow or controlled mailbox contract; the production checklist and prior deployment-target preflight require external target/identity configuration.
rejected_hypotheses:
  - Cloudflare edge remains broken; final WAF/Bot, public E2E and HSTS evidence passed.
  - A generic forgot-password HTTP 200 proves delivery; it proves only enumeration-safe request handling.
  - A log or in-memory mail transport proves external delivery; the implementation explicitly rejects log transport and array delivery has no external effect.
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
  - command: controlled password-recovery delivery
    result: BLOCKED
    evidence: no controlled identity/mailbox or approved delivery-capable target is available without inventing or exposing credentials
blockers:
  - provision a controlled test identity and delivery mailbox through an approved protected environment; do not paste credentials or reset tokens into chat or Git
next_action: Provision protected target URL, controlled test email and delivery-verification access for the intended environment, then execute a bounded password-reset request, verify receipt and link host, complete the reset, confirm single-use and session-revocation behavior, sanitize evidence, and archive this task. Full production go-live remains separate under Issue #91.
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
