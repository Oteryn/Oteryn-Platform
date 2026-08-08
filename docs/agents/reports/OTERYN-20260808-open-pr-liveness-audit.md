# Open pull-request liveness audit — 2026-08-08

## Result

`AUDIT_COMPLETE_WITH_FINDINGS`

Audited protected `main@5d8a9bcd46ca45984bb45e467d4837ad8f541b59` and all six pull requests that were open at invocation start. Four PRs had evidence-backed current/intentional dispositions at observation time. Two material contradictions were proven and routed to deduplicated P1 remediation Issues #885 and #886.

During terminal validation, PR #881 completed and merged as `4043edfaf67b9489d050d70e6fb7e32f4bf149c2`. The audit delivery was reconstructed directly on that new protected `main`; the concurrent architecture merge does not alter either finding and converts PR #881 from invocation-time current work to terminal delivered work.

No application, runtime, workflow, deployment, Cloudflare, Synology, credential, production or external-repository mutation was performed.

## Audit matrix

| PR | Classification | Evidence-backed disposition | Durable owner |
|---:|---|---|---|
| #882 | current active delivery | Issue #244 has a current active claim on deterministic branch `repair/issue-244`; the PR implements the accepted administrator homepage-template selector scope. | Issue #244 / PR #882 |
| #881 | current at observation; merged during validation | Issue #880 and its branch-local task owned the native runtime-status projection boundary; PR #881 subsequently merged during this audit. | merged PR #881 |
| #541 | intentional external wait | Public edge/HSTS repair is already reconciled; the only remaining public-domain criterion is owner-observed staging password-recovery evidence. | PR #541 |
| #338 | intentional dependency hold | Programme #330 explicitly requires separate Canary schema 1.3 producer compatibility before the inactive Platform consumer may merge. | Issue #330 / PR #338 |
| #391 | material authority/lifecycle conflict | The live-reference task still routes final compatibility handoff toward historical `blakinio/otclient` even though ADR 0031 and completed Issue #864 record Oteryn-v2 as current native authority. Preserve the safe harness; reconcile target authority and PR lifecycle. | `OPA-GOV-0029`, Issue #886 |
| #405 | material evidence/lifecycle conflict | The production-gate branch still carries August 1 WWW/TLS/redirect/HSTS failures and asks for a Cloudflare audit/apply that later PR #516/PR #541 evidence superseded. `PRODUCTION_PROVEN=false` remains correct under Issue #91. | `OPA-GOV-0028`, Issue #885 |

## Finding OPA-GOV-0028 — PR #405 retains superseded public-edge blockers

**Severity:** HIGH  
**Priority:** P1  
**Confidence:** HIGH  
**Evidence:** PROVEN  
**Issue:** #885

PR #405 remains a draft historical production-gate evidence branch. Its current checkpoint still treats WWW Cloudflare 403, Gateway TLS failure, missing HTTP redirect and `max-age=0` HSTS as current blockers, and its next action requests a separate Cloudflare zone-edge audit/apply.

Those statements were valid for the August 1 observation generation, but later durable evidence changed the state:

- PR #516 records guarded WAF/Bot/public-edge/HSTS repair and independent trusted-main verification;
- PR #541 independently consumes that later evidence and treats public-edge/HSTS work as complete;
- Issue #877 separately owns stale Cloudflare-verification task evidence, so that work was not duplicated here.

Issue #91 remains open and continues to require direct evidence tied to an exact deployed production release. Therefore the audit does **not** claim production readiness. The defect is the stale PR generation and obsolete privileged next action.

## Finding OPA-GOV-0029 — PR #391 routes handoff to historical OTClient authority

**Severity:** HIGH  
**Priority:** P1  
**Confidence:** HIGH  
**Evidence:** PROVEN  
**Issue:** #886

PR #391 contains useful, bounded synthetic/no-network Linux research-harness work and remains blocked from official-service execution by explicit local prerequisites. Those safety boundaries are not the finding.

The conflict is its final compatibility/handoff authority. The task and plan still route future implementation analysis toward `blakinio/otclient`, Platform and Canary as the project-owned target chain. Since the task began, accepted ADR 0031 split target Native Oteryn-v2 Integration from Legacy Canary Compatibility. Completed Issue #864 records the Rust-client migration from historical `blakinio/otclient` into `blakinio/Oteryn-v2/apps/client` and classifies legacy correspondence as reference/reconciliation evidence rather than final native conformance.

The safe repair is to preserve the proven harness and its strict credential/anti-cheat boundaries while reconciling the target repository authority and PR lifecycle. No official login or external-repository write is needed for that repair.

## Intentional retained PRs

### PR #882

Current active work. Issue #244 records a live high-risk/P2 feature claim, exact exclusive paths and the deterministic `repair/issue-244` branch. No stale-owner or authority contradiction was found.

### PR #881

At observation time this was current active architecture work with exact Issue #880 ownership. It merged during terminal validation as `4043edfaf67b9489d050d70e6fb7e32f4bf149c2`; no audit finding is warranted from that normal lifecycle transition.

### PR #541

Intentional waiting state, not stale merely because it is old. Later edge/HSTS evidence has already been incorporated. Remaining progress depends on owner-observed staging mailbox evidence, so no duplicate finding or PR was created.

### PR #338

Intentional cross-repository compatibility hold. Programme #330 defines consumer-first rollout and requires a separate Canary schema 1.3 producer before merge. The Platform consumer's completed validation does not remove that dependency.

## Prior baseline reconciliation

The 2026-08-02 production-completion baseline correctly required every retained PR to have a current action/dependency and to be revisited at later programme barriers. This audit does not treat that historical disposition file as live authority. It re-evaluated the six invocation-start PRs against evidence available on 2026-08-08.

The changed dispositions for #391 and #405 are caused by later accepted architecture/evidence, not by PR age alone.

## Safety and validation

Runtime/browser E2E: `NOT_APPLICABLE` because the audit delivery changes only audit/governance documentation.

Production/staging mutation: `NOT_RUN`.  
Credential/secret use: `NOT_RUN`.  
Cloudflare/Synology mutation: `NOT_RUN`.  
External-repository write: `NOT_RUN`.

Final delivery requires exact-head Agent Governance and repository-selected CI on PR #884 after reconstruction on current protected `main`, full changed-file review and zero unresolved material review threads before merge.
