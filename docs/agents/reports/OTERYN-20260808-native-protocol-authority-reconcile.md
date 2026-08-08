# Native protocol authority reconciliation — 2026-08-08

## Scope

Architecture-only review of the current Oteryn Platform native gameplay-protocol documents after acceptance of ADR 0031.

No runtime code, database schema, workflow behavior, external repository, deployment or production state was changed by this review.

## Question

Does current `main` consistently route target native gameplay protocol and admitted-session authority to Oteryn-v2, or can the pre-ADR-0031 Platform/Otheryn producer package still be read as the canonical native contract?

## Evidence classification

### PROVEN

1. `docs/architecture/ARCHITECTURE_AUTHORITY.md` gives accepted ADRs higher precedence than operation-specific contracts and requires lower-ranked conflicts to be recorded rather than interpreted locally.
2. ADR 0031 is accepted and assigns:
   - Platform: `AccountId`, Identity, OAuth/PKCE, Game Login Tickets, World Registry and Gateway pre-admission/control-plane orchestration;
   - Oteryn-v2 game/native authority: final gameplay admission, authoritative admitted gameplay session/lease/fencing, gameplay state, native persistence and `protocol-oteryn` gameplay semantics.
3. `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md` explicitly classifies the existing Platform native protocol contract/Game Session v2 producer as transitional historical evidence and says exact session/lease details remain follow-up contract work.
4. Before this repair, `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md` still declared `Status: NORMATIVE` and stated that it was the canonical contract governing Platform, Game Gateway, Otheryn and the Rust client. It specified Game Session v2 admission/session states, binding, parser, command and reconciliation semantics as if they were current native authority.
5. Before this repair, `docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md` still described an Otheryn Game Session v2/readiness/native listener as the remaining target consumer package and provided rollout/rollback instructions in that topology.
6. `docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md` already classifies the historical Platform/Otheryn/OTClient path as compatibility/reconciliation evidence and blocks final native verification on Oteryn-v2 runtime/admission authority plus separately authorized Platform reconciliation.
7. Repository validators bind the historical protobuf digest, fixtures and defensive invariants to this package. Those checks prove artifact consistency; they do not have accepted-ADR authority.

### DERIVED

- The old contract text was a material authority-routing defect even though ADR 0031 would win a careful precedence analysis: an agent entering through the contract or producer guide could still implement or verify the wrong target topology.
- The safest repair is not to delete the historical schema/fixtures/producer evidence. It is to remove the current normative claim, preserve immutable historical markers for reproducibility, and make Oteryn-v2 reconciliation an explicit prerequisite before any future producer activation.
- No new ADR or owner decision is required because ADR 0031 already resolves the ownership question. This package only makes lower-level documents conform to that accepted authority.

### UNKNOWN

The exact future Oteryn-v2 admission credential, admitted-session generation/lease/fencing envelope, reconnect semantics, parser limits and protocol bytes are not established by Oteryn Platform current authority. They remain `UNKNOWN` here and must not be reconstructed from the historical Platform/Otheryn package.

## Alternatives considered

### A. Leave the old contract unchanged and rely only on ADR precedence

Rejected. Correct precedence exists, but the stale self-declared `NORMATIVE` contract and rollout document remain a high-risk routing trap in an authentication/session boundary.

### B. Delete the old contract, protobuf and fixtures

Rejected. The disabled producer is merged repository history and remains useful transition/audit evidence. Deleting its current reproducibility surface would make reconciliation harder and could obscure what the Platform producer actually emits.

### C. Reclassify the package as historical/transitional and preserve validator markers

Selected. This removes competing authority while keeping exact historical artifact identity and fail-closed regression evidence available for future comparison.

## Applied reconciliation

### Historical contract

`docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md` now:

- declares `HISTORICAL / TRANSITIONAL RECONCILIATION INPUT — NOT CURRENT NATIVE AUTHORITY`;
- routes current authority to ADR 0031 and `OTERYN_V2_INTEGRATION_ARCHITECTURE.md`;
- states that Oteryn-v2 owns final admission/session/lease/fencing and gameplay protocol semantics;
- states that historical PR #542 bytes/claims are not Oteryn-v2 conformance evidence;
- preserves the schema digest and exact validator-required historical invariants/bounds so the old artifact remains reproducible;
- requires explicit field-by-field reconciliation against an accepted Oteryn-v2 contract before any future producer activation.

### Producer operations record

`docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md` now:

- declares the producer historical, complete, disabled and reconciliation-only;
- removes the old Otheryn-consumer rollout as current target guidance;
- preserves the historical native tuple/schema digest for artifact validation;
- separates reusable Platform-owned pre-admission principles from game-owned Oteryn-v2 authority;
- makes exact Oteryn-v2 contract evidence, compatibility proof, E2E, environment evidence, rollback and activation authority mandatory before any future enablement.

## Result

`RESOLVED_BY_EXISTING_AUTHORITY`

No new architecture decision was created. Issue #874 records the defect; ADR 0031 remains the canonical durable decision.

## Validation disposition

- Runtime/application build: `NOT_APPLICABLE` — documentation-only architecture reconciliation.
- Browser/runtime E2E: `NOT_APPLICABLE` — no executable user or integration behavior changed.
- Required final evidence: exact-head Agent Governance, protected CI, path-routed native protocol artifact validation/audits, full changed-file review and resulting-main verification after merge.
