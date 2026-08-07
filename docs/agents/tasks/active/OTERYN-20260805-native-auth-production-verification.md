---
task_id: OTERYN-20260805-native-auth-production-verification
repository: blakinio/Oteryn-Platform
execution_mode: verification_only
branch: none
pull_request: none
status: blocked
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
search_first:
  - issue #864
  - issue #565
  - Platform PR #542
  - current Oteryn-v2 foundation execution status
optional_reads: []
---

# OTERYN-20260805-native-auth-production-verification

## Goal

Preserve the unresolved production-safety verification gates without retaining obsolete runtime ownership or treating the pre-cutover OTClient/Canary chain as final native authority. Keep legacy compatibility proof separate from final Oteryn-v2 native admission/protocol verification, and keep production activation disabled until the applicable exact-revision, network, TLS, credential-injection and production-authority gates are proven.

## Current authority split

### Native target

The current Platform target architecture is the Rust client -> Oteryn Platform Identity -> Game Gateway pre-admission -> Oteryn-v2 game boundary -> `protocol-oteryn` flow defined by `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md` and current Oteryn-v2 foundation authority.

Oteryn-v2 FND-02 `protocol-oteryn` v1 is accepted and merged by Oteryn-v2 PR #94 as `769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202`, but runtime/component/E2E implementation is explicitly not implemented or authorized by FND-02. FND-03 is the next ordered runtime-execution gate and FND-04 admission/Game Session/lease/reconnect remains behind its own contract.

The historical Platform native gameplay contract at `c0b8703d326a04b43ae8e06f6192b0cb91c859b7` is reconciliation input, not final Oteryn-v2 protocol authority.

### Compatibility evidence

The delivered Platform/Game Gateway/Canary-compatible chain remains useful only as bounded migration/compatibility evidence when that compatibility path is intentionally exercised. A staging E2E for that chain is therefore preserved conditionally, but it must be labelled:

- `COMPATIBILITY_ONLY`;
- `NON_PRODUCTION_NATIVE`;
- `NOT_OTERYN_V2_CONFORMANCE`.

A successful compatibility E2E does not satisfy or unblock final Oteryn-v2 native production verification. If the compatibility path is not selected for an authorized staging/deployment exercise, absence of that compatibility E2E does not block Oteryn-v2 conformance.

Historical `blakinio/otclient` Rust correspondence and Otheryn/Canary native-protocol correspondence are compatibility/reconciliation evidence only after the client/runtime authority cutover. They are not prerequisites for final Oteryn-v2 conformance.

## Acceptance criteria

- [x] Platform PR #542 is recorded as terminal merged producer evidence: `93b122c29ba774c71ff6921cd5b4c5c57c089b61`; native advertisement/production activation remained disabled.
- [x] Historical OTClient and Otheryn/Canary correspondence is classified as historical compatibility/reconciliation evidence, not final Oteryn-v2 conformance.
- [x] The legacy Platform/Gateway/Canary E2E decision is explicit: preserve it only as conditional bounded compatibility evidence with the labels above.
- [ ] If an authorized staging/deployment plan intentionally exercises the legacy compatibility chain, run that bounded compatibility E2E on its selected exact revisions and record the result without promoting it to native conformance evidence.
- [ ] Final native production verification is executed only after current Oteryn-v2 runtime/admission gates and the separately authorized Platform reconciliation produce an implementable exact-revision native chain.
- [ ] Direct deployed evidence proves the applicable private-network boundaries, TLS certificate/hostname validation, service-credential injection and running revisions for the production candidate chain.
- [ ] Production activation remains disabled until every applicable native production gate passes and explicit production authority exists.
- [x] This record owns no runtime, route, contract, workflow, secret, deployment or external-repository path.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md
modules:
  - verification evidence only
dependencies:
  - Platform PR #124 merged as 53158217a6c6017230301cf4daa783b04fcc13d5
  - Platform PR #542 merged as 93b122c29ba774c71ff6921cd5b4c5c57c089b61 with native producer disabled
  - Platform ADR 0031 / OTERYN_V2_INTEGRATION_ARCHITECTURE define the current native integration target
  - Oteryn-v2 FND-02 accepted and merged as 769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202
  - Oteryn-v2 FND-03 runtime contract is the next ordered gate
  - Oteryn-v2 admission/Game Session/lease/reconnect work remains behind later separately authorized gates
  - a separately authorized Platform reconciliation is still required before final Gateway/World Registry/session-offer conformance can be claimed against accepted FND-02
blockers:
  - final Oteryn-v2 runtime/admission implementation and exact production-native revisions do not yet exist under current accepted gates
  - Platform Gateway/World Registry/session-offer structures have not yet been separately reconciled and accepted against final Oteryn-v2 FND-02 authority
  - authorized deployed environment evidence for private networking, TLS, service credentials and running revisions is not attached
  - production activation authority is separate and absent
cross_repository_tasks:
  - none claimed by this record
```

## Verification tracks

### Track A — legacy compatibility staging proof

Status: `CONDITIONAL / NOT_RUN`.

Run only if an authorized staging/deployment plan intends to keep exercising the delivered Canary-compatible path. Record exact selected revisions, environment and result. This track is migration/compatibility evidence only and cannot become an Oteryn-v2 production-readiness gate by implication.

### Track B — final Oteryn-v2 native production proof

Status: `BLOCKED`.

Do not substitute historical Canary, Otheryn or `blakinio/otclient` revisions for the final native chain. Final verification waits for:

1. current Oteryn-v2 foundation/runtime/admission gates to authorize and deliver the native server/client path;
2. separately authorized Platform reconciliation of Gateway/World Registry/session-offer semantics against the accepted Oteryn-v2 authority;
3. exact terminal revisions for every component actually selected for the native production candidate;
4. authorized end-to-end native admission/protocol validation;
5. deployed private-network, TLS, service-credential injection and running-revision evidence;
6. explicit production activation authority.

## Production safety invariants

These requirements survive the architecture cutover wherever they apply to the selected production chain:

- internal service boundaries are private/restricted according to the accepted deployment topology rather than publicly exposed by accident;
- TLS validates the expected certificate identity and hostname/server name at every required service/game endpoint;
- service credentials are injected from the approved secret-management path and are not embedded in source or logs;
- evidence identifies the exact running revisions rather than inferring deployment state from repository `main`;
- repository tests alone do not prove deployed production readiness;
- production activation is a separate explicitly authorized operation and is not implied by this verification record.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T01:31:00+02:00
head: 01f89a38fdc2c5dc96799b77a981ed067cd17a32
branch: none
pr: none
status: blocked
context_routes:
  - auth-identity
  - oteryn-v2-integration
  - deployment-operations
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md
proven:
  - Platform repository hardening PR #124 merged as 53158217a6c6017230301cf4daa783b04fcc13d5 from final head b757b2f5d6812467527507c20fe25542429a01d4.
  - Platform native producer PR #542 is terminal and merged as 93b122c29ba774c71ff6921cd5b4c5c57c089b61; its native advertisement and production activation remained disabled.
  - Platform OTERYN_V2_INTEGRATION_ARCHITECTURE defines the current native target as Platform pre-admission feeding Oteryn-v2 game-owned authoritative admission and protocol-oteryn; Canary is compatibility only.
  - Oteryn-v2 FND-02 is accepted and merged as 769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202, and its current status explicitly says runtime/component/E2E is not implemented or authorized by FND-02.
  - Oteryn-v2 current execution status makes FND-03 the next ordered runtime gate and keeps admission/Game Session/lease/reconnect behind later dedicated gates.
  - The historical Platform gameplay-protocol revision is classified by Oteryn-v2 as RECONCILIATION_INPUT_ONLY.
  - The migrated Rust client authority is Oteryn-v2/apps/client; historical blakinio/otclient correspondence is not final native authority.
  - The delivered Canary-compatible path can still provide bounded migration/staging compatibility evidence when intentionally exercised, but that evidence is not Oteryn-v2 conformance.
derived:
  - Exact legacy Canary/OTClient revisions are no longer prerequisites for final native Oteryn-v2 production verification.
  - Final native verification cannot be performed until current Oteryn-v2 runtime/admission gates and a separate Platform reconciliation produce the real native candidate chain.
  - Deployed private-network, TLS, service-credential injection and exact-running-revision proof remain applicable safety gates, but must be collected against the chain actually selected for production.
unknown:
  - whether an authorized staging plan will intentionally exercise the legacy compatibility chain and therefore require Track A evidence
  - exact future terminal revisions of the implemented Oteryn-v2 native client/server/admission chain
  - exact future Platform reconciliation revision selected for native production
  - authorized native E2E result for that future exact-revision chain
  - deployed private-network ingress/firewall topology for the selected production candidate
  - deployed TLS certificate and hostname-verification state for the selected production candidate
  - secret-manager injection and running-revision evidence for the selected production candidate
  - explicit production activation authority
conflicts: []
first_failure:
  marker: verification authority drift after Oteryn-v2 cutover
  evidence: Issue #864 proves the verification record still used the pre-cutover OTClient -> Gateway -> Canary dependency model after Oteryn-v2 became the canonical native target.
rejected_hypotheses:
  - A successful legacy compatibility E2E proves Oteryn-v2 native conformance.
  - Final Oteryn-v2 verification must wait for exact historical Canary and blakinio/otclient revisions.
  - FND-02 acceptance means the native runtime/admission path is already implemented or production-ready.
  - Repository CI alone can prove deployed private-network, TLS, credential or running-revision state.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md
validation:
  - command: live Platform PR #542 reconciliation
    result: PASS
    evidence: merged as 93b122c29ba774c71ff6921cd5b4c5c57c089b61 with native producer disabled.
  - command: current Platform architecture authority reconciliation
    result: PASS
    evidence: OTERYN_V2_INTEGRATION_ARCHITECTURE makes Oteryn-v2 the native game/admission/protocol target and Canary compatibility an anti-corruption/migration path.
  - command: current Oteryn-v2 foundation authority reconciliation
    result: PASS
    evidence: FND-02 merged as 769ecd2ce2dfe0a7644d8dc1d67c54d40da5d202; runtime/E2E not authorized; FND-03 is next and admission remains later-gated.
  - command: reconciliation E2E
    result: NOT_APPLICABLE
    evidence: this change only corrects verification authority/lifecycle metadata and does not execute or modify a runtime path.
blockers:
  - final native runtime/admission implementation and separately authorized Platform reconciliation are not yet available
  - deployed safety evidence and production activation authority are absent
next_action: Keep this record verification-only and blocked. Run Track A only if a compatibility staging plan is explicitly selected; otherwise wait for current Oteryn-v2 runtime/admission delivery plus a separately authorized Platform reconciliation, then verify the exact native production candidate and deployed network/TLS/credential/revision state before any production activation.
```
