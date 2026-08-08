# Native runtime-status projection boundary review — 2026-08-08

## Scope

Architecture-only review of the Platform World Registry / Game Gateway / LiveOps boundary for native Oteryn-v2 runtime health, readiness, lifecycle and freshness observations.

No runtime code, schema, workflow, deployment, production state or external repository was changed.

## Question

Can Platform define a safe native runtime-status consumer boundary from already accepted authority without inventing the unfinished Oteryn-v2 OPS-CHANNEL-01/FND producer transport?

## Evidence classification

### PROVEN

1. ADR 0029 makes Platform the canonical owner/issuer of `WorldId` and `ChannelId`, World Registry topology identity/configured routing policy and authorized topology projection, while explicitly separating GameNode process identity and writer-generation/fencing authority.
2. ADR 0031 makes Oteryn-v2 authoritative for game runtime/gameplay source facts and requires Platform native integrations to use explicit contracts rather than game persistence/table coupling.
3. `OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md` says route/readiness metadata is replaceable topology data and that unknown required readiness fails closed where readiness is mandatory.
4. `WORLD_REGISTRY_CONTRACT.md` implements a Canary-compatible persisted status/login gate and, before this repair, explicitly left the future relationship between runtime health, persisted `status` and live readiness unresolved.
5. `MODULE_CATALOG.md` assigns authoritative time-sensitive world/service status, freshness, maintenance/service history and explicit stale/unavailable behavior to planned LiveOps; it forbids fabricated offline/zero state.
6. The focused `OTERYN_V2_INTEGRATION_ARCHITECTURE.md` previously listed `World/channel runtime-status → Platform World Registry/LiveOps contract` as an unresolved P1 architecture item and already required PublicGameData projections to define producer, applicability, observation/revision, freshness, stale/unavailable semantics, privacy, rebuild/reconciliation and failure behavior.
7. Read-only Oteryn-v2 ADR-0009 is accepted foundation direction and distinguishes GameNode health/readiness/capacity from ChannelRuntime lifecycle and durable Channel identity. It says channels become routable only after readiness/revision checks and Gateway stops new routing to affected channels after unhealthy/suspected GameNode failure.
8. The same Oteryn-v2 ADR defers exact GameNode registration/health/readiness/capacity reporting, orchestration and recovery details to `OPS-CHANNEL-01`, with FND-03/FND-04 retaining execution/admission/session authority.
9. Fresh overlap search found no open bounded Oteryn-Platform Issue or PR already owning this exact runtime-status projection architecture gap; Issue #880 owns the work.

### DERIVED

- Platform can freeze the **consumer semantics** now: configured Platform policy and observed runtime facts are independent authorities; native readiness requires fresh applicable Oteryn-v2 evidence scoped to canonical WorldId/ChannelId; stale/unavailable/invalid or superseded-owner evidence fails closed for new admission.
- Platform cannot honestly freeze producer-specific JSON/protobuf fields, exact heartbeat cadence/TTL, health algorithm, ownership-generation encoding or capacity thresholds because the owning Oteryn-v2 contracts remain deferred.
- A Platform status cache/read model can improve availability and presentation but cannot become the runtime source of truth or extend observation freshness silently.
- Public status truth and Gateway admission readiness need different consumer views of the same authoritative observations: Gateway must fail closed, while public presentation must represent stale/unavailable evidence as uncertain rather than fabricate `offline` or zero.
- A world-level public status must aggregate the current canonical channel set explicitly; one failed/unavailable channel cannot automatically define the whole world as offline.

### UNKNOWN

The following remain outside Platform authority until their owning Oteryn-v2 contracts/evidence are accepted:

- exact status event/API schema and transport;
- exact GameNode/ChannelRuntime status encoding;
- exact health/readiness computation;
- exact heartbeat/reporting cadence and TTL durations;
- exact ownership-generation/fencing representation;
- exact capacity thresholds/overload rules;
- exact production deployment topology.

## Alternatives considered

### A. Keep native runtime readiness as a later implementation detail

Rejected. The existing architecture already requires readiness/freshness and identifies this contract as P1. Leaving semantics undefined invites configuration/runtime truth collapse in a security-sensitive admission path.

### B. Reuse current persisted `GameWorldStatus` as native runtime truth

Rejected. `status=online` and `login_enabled=true` are Platform compatibility/configuration state. They cannot prove that the current Oteryn-v2 runtime owner is healthy, current, compatible and ready.

### C. Copy Oteryn-v2 ADR-0009 lifecycle states into a Platform-owned producer protocol

Rejected. The lifecycle direction is useful read-only authority, but exact producer transport, health reporting and orchestration remain Oteryn-v2-owned follow-up contracts.

### D. Define a Platform consumer semantic envelope and keep producer encoding deferred

Selected. This resolves the Platform architecture dependency while preserving source ownership and external contract authority.

## Applied architecture

### New focused contract

`docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md` now defines:

- canonical `WorldId + ChannelId` observation scope;
- authoritative Oteryn-v2 runtime/orchestration source ownership;
- separation of configured Platform policy from observed runtime state;
- semantic observation evidence requirements without freezing wire encoding;
- consumer evidence states `fresh`, `stale`, `unavailable`, `invalid` distinct from runtime lifecycle values;
- fail-closed new-admission evaluation requiring fresh readiness/revision/current-owner evidence in addition to Platform policy;
- stale-owner/generation rejection and failover ordering rules;
- world-level aggregation constraints;
- truthful LiveOps/public stale/unavailable behavior;
- cache/read-model non-authority and rebuild/reconciliation requirements;
- security/redaction/observability requirements;
- exact deferred producer/implementation details and future validation requirements.

### World Registry reconciliation

`docs/contracts/WORLD_REGISTRY_CONTRACT.md` now explicitly classifies its persisted status/login model as current Canary compatibility/configuration state for native-runtime purposes and routes native status/readiness semantics to the new focused contract. It keeps current implementation evidence unchanged and does not claim a runtime integration exists.

### Focused v2 architecture reconciliation

`docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md` now records native runtime-status/readiness source ownership and consumer flow, adds the boundary to admission/public projection semantics, and moves the former P1 runtime-status item into the resolved-focused-boundaries section. Exact producer transport/implementation remains deferred.

## Result

`RESOLVED_BY_EXISTING_AUTHORITY`

No new owner product choice or new ADR is required for the Platform consumer semantics. Exact Oteryn-v2 producer implementation remains a separately governed external contract task.

## Validation disposition

- Runtime/application build: `NOT_APPLICABLE` — architecture/documentation only.
- Browser/runtime E2E: `NOT_APPLICABLE` — no executable behavior or environment changed.
- Required final evidence: exact-head Agent Governance, repository-selected protected CI, full changed-file review, zero unresolved material findings, merge and resulting-main verification.
