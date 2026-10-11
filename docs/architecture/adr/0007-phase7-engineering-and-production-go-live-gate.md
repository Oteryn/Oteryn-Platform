# ADR 0007 — Separate Phase 7 engineering completion from the Production Go-Live Gate

## Status

Accepted — 2026-07-21

## Context

Phase 7 is named **Production hardening and operations**. Its roadmap deliverables are engineering and operational capabilities owned by the repository and its documented deployment procedures: production-safe configuration guardrails, security headers/CSP, dependency scanning, structured logging/request correlation, least-privilege database verification, runtime Redis boundary validation, mail validation, deployment/rollback procedures, backup/restore procedures, incident/recovery runbooks and critical-flow validation.

Those mechanisms have now been implemented and exercised in a controlled production-like environment. PR #63 merged the exact-SHA `Phase 7 Production-Like Validation` workflow. Its final PR head `7842f78ec4ac2d07d3800ffe8bde9809b055822d` passed Production-Like Validation run #9, required CI #759 and Agent Governance #679. The evidence remains correctly classified as `STAGING_PROVEN` and explicitly does not prove the final production environment.

The existing Phase 7 roadmap exit gate delegates completion to `docs/operations/PRODUCTION_READINESS_CHECKLIST.md`. That checklist intentionally requires facts that can exist only after or during a real production deployment: production DNS/edge/TLS/origin/firewall state, actual production DB/Redis principals and network paths, provider-specific deployment controls, production mail and observability, production backup schedule and dated restore evidence, exact deployed SHAs and final production smoke tests.

As a result, the current model conflates three different questions:

1. **Engineering completion** — has the project delivered and tested the Phase 7 hardening/operations mechanisms?
2. **Production-like validation** — have those mechanisms been exercised credibly in a controlled environment without claiming production state?
3. **Production go-live verification** — has the actual deployed production environment been directly verified and approved for go-live?

Keeping all three inside one Phase 7 status leaves a completed engineering phase permanently `IN PROGRESS` solely because an external deployment has not yet occurred. Marking the phase complete without a durable decision, however, would contradict the existing explicit roadmap/checklist wording.

## Decision

### 1. Phase 7 completion represents engineering/hardening completion

Phase 7 is complete when the repository has delivered the required production-hardening and operations mechanisms and they have passed the required repository validation plus controlled production-like validation appropriate to the mechanism.

Phase 7 completion does **not** mean:

- the application is deployed to final production;
- the production environment is production-ready;
- any final production fact is `PRODUCTION_PROVEN`;
- staging restore timing establishes production RTO/RPO;
- staging DB/Redis principals prove production grants/ACLs;
- production DNS, Cloudflare, TLS, origin exposure or firewall state is correct.

Under this boundary, merged PR #63 closes the Phase 7 engineering/hardening scope and Phase 7 may be marked `COMPLETE`.

### 2. Production-like readiness remains `STAGING_PROVEN`

`docs/operations/PRODUCTION_LIKE_VALIDATION_EVIDENCE.md` remains the authoritative record of controlled-environment evidence.

Its evidence taxonomy remains authoritative for environment proof:

- `STAGING_PROVEN` — directly demonstrated in controlled production-like validation;
- `PRODUCTION_PROVEN` — directly demonstrated in the final production environment;
- `UNKNOWN` — not yet proven for the stated environment.

Repository-only labels such as `REPO-PROVEN` remain supporting control evidence in the production checklist, but they never promote an environment-specific item to `PRODUCTION_PROVEN`.

No staging result may be relabeled as production evidence.

### 3. The existing production readiness checklist becomes the fail-closed Production Go-Live Gate

`docs/operations/PRODUCTION_READINESS_CHECKLIST.md` remains the single durable checklist for final production verification and is reclassified as the authoritative **Production Go-Live Gate**.

The gate status is independent from the Phase 7 engineering status.

Required project status model:

- `Phase 7 — COMPLETE`;
- `Production Readiness — STAGING_PROVEN`;
- `Production Go-Live Gate — PENDING PRODUCTION VERIFICATION`;
- `Production Verification — REQUIRED BEFORE GO-LIVE`.

The go-live gate is fail closed. It cannot become `PASS` while any mandatory final-production verification item remains `UNKNOWN` or otherwise lacks direct production evidence.

An owner may explicitly accept an eligible operational or security risk where repository policy permits, but risk acceptance:

- is a separate decision, not an evidence classification;
- does not convert `UNKNOWN` or `STAGING_PROVEN` into `PRODUCTION_PROVEN`;
- cannot be used to claim that an unverified production fact was verified;
- cannot bypass mandatory proof of the actual deployed release identity and the core production environment boundaries required by the go-live gate.

### 4. Final production verification remains mandatory before go-live

The Production Go-Live Gate continues to require direct production evidence for the applicable launch scope, including:

- exact deployed Oteryn Platform SHA and relevant legacy engine/login-server versions;
- production DNS, Cloudflare/edge/WAF/Access and TLS behavior;
- direct-origin exposure and effective ingress firewall/reverse-proxy restrictions;
- production Platform and legacy engine DB topology, network isolation and effective grants;
- production runtime Redis endpoint, ACL, network and TLS state;
- production session/cache/queue topology and worker behavior where applicable;
- production mail provider/domain/delivery monitoring;
- production logs/metrics/alerts/retention/access/on-call routing;
- actual provider deployment/migration/rollback mechanism and authorized emergency rollback path;
- production backup policy/schedule and dated production restore evidence;
- final production health/readiness and critical smoke/E2E checks against the exact deployed SHA.

These items remain `UNKNOWN` until directly proven in production.

### 5. Authoritative game-login readiness is launch-scope dependent and remains separate

The authoritative Platform game-login bridge is not retroactively made part of Phase 7 engineering completion.

If Platform-originated game login is required for the selected launch scope, the Production Go-Live Gate remains blocked until the separately authorized cross-repository game-login requirement is resolved and its required end-to-end properties are proven.

This decision does not authorize any legacy engine/login-server repository write.

### 6. Material candidate changes may require revalidation without reopening the historical phase

A material change to the production candidate, deployment contract, security boundary or critical integration after Phase 7 completion may require the relevant repository/staging validation to be rerun for the new candidate.

That requirement belongs to release/go-live verification. It does not automatically change the historical Phase 7 engineering status back to `IN PROGRESS` unless the project explicitly reopens the phase because a new engineering gap is discovered.

## Consequences

- Phase 7 can accurately close as an engineering/hardening phase after merged production-like validation.
- Final production deployment remains blocked from a go-live `PASS` until the separate Production Go-Live Gate is satisfied.
- Production claims remain fail closed and evidence-specific.
- The project no longer needs to keep an engineering phase open solely because final infrastructure is unavailable.
- There is one production checklist, not separate competing readiness and go-live checklists.
- Staging restore measurements remain staging-only recovery evidence and cannot be used as production RTO/RPO.
- Actual production grants, ACLs, network boundaries, TLS and provider controls must still be verified directly.

## Rejected alternatives

### Keep Phase 7 `IN PROGRESS` until a real production deployment exists

Rejected. This makes an engineering phase status depend indefinitely on an external deployment event even after all defined hardening mechanisms and controlled validation are complete. The actual production verification remains mandatory, but it is better represented as a separate go-live gate.

### Mark the system production-ready because production-like staging passed

Rejected. Controlled staging proves mechanisms and procedures only. It cannot prove final production DNS, TLS, firewall, provider controls, credentials, topology, backup schedule, observability or deployed SHA.

### Treat owner risk acceptance as production evidence

Rejected. Risk acceptance is a governance decision and cannot fabricate `PRODUCTION_PROVEN` evidence.

### Create a second independent go-live checklist

Rejected. Duplicated checklists would drift. The existing `docs/operations/PRODUCTION_READINESS_CHECKLIST.md` is retained and reclassified as the single fail-closed Production Go-Live Gate.

## Follow-up

Before production go-live:

1. identify the exact production candidate/deployed SHA;
2. execute only the applicable final production verification items in `docs/operations/PRODUCTION_READINESS_CHECKLIST.md` and `docs/operations/PRODUCTION_LIKE_VALIDATION_EVIDENCE.md`;
3. record direct production evidence without secrets;
4. keep every unproven production fact `UNKNOWN`;
5. permit go-live `PASS` only when the mandatory gate is satisfied for the selected launch scope.
