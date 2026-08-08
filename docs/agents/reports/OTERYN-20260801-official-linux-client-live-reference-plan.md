# Official Tibia Linux client live-reference plan

## Status

`HISTORICAL RESEARCH PLAN / CURRENT AUTHORITY RECONCILED / LIVE EXECUTION NOT READY`

This plan preserves the safe research design and synthetic harness delivered on historical PR #391. It no longer treats historical `blakinio/otclient` or Canary as the current native implementation target.

Current native authority is governed by ADR 0031:

- Oteryn Platform owns Identity, OAuth/PKCE, Game Login Ticket, Gateway pre-admission/routing and Platform-side interoperability workflow;
- `blakinio/Oteryn-v2` owns the native Rust client, native game server and `protocol-oteryn` implementation authority;
- historical OTClient, Canary and official Tibia behavior are reference/compatibility evidence only.

The live official service remains a reference system, never an implementation or architecture authority.

## Purpose

Preserve a bounded capability to observe the lawfully obtained, unmodified official Tibia Linux client under strict owner authorization and convert safe observations into evidence for current Oteryn architecture.

The capability is intentionally separated into:

1. synthetic/no-network harness validation;
2. official package/component validation without authentication;
3. owner-gated bounded live observation;
4. evidence classification and current-authority handoff.

Only phase 1 is proven. Phases 2 and 3 remain blocked.

## Safety and authorization boundary

Authorized only after its phase-specific gates are satisfied:

- launch the unmodified official Linux client;
- use only an account owned and explicitly designated by the repository owner;
- authenticate only through manual local entry or another approved non-logged ephemeral mechanism;
- collect bounded local process/window/timing/filesystem/network metadata without modifying, decrypting, injecting into or replaying official traffic;
- retain raw sensitive evidence only on a private encrypted volume outside Git;
- publish only redacted text evidence, hashes and non-sensitive manifests.

Never authorized:

- patching, hooking, injection, process-memory modification or BattlEye/anti-cheat bypass;
- traffic modification, credential interception, replay or protocol abuse;
- gameplay automation, farming, combat, trade or interaction with other players;
- redistribution of official binaries/assets;
- storing credentials/session material in ChatGPT, GitHub, shell arguments, ordinary environment variables, logs or artifacts;
- converting one observed official session into a claim of complete Oteryn-v2 compatibility.

## Research environment

Preferred future environment:

- dedicated interactive Linux x86-64 research host;
- disposable/snapshotted OS where practical;
- dedicated non-privileged user;
- encrypted private evidence volume outside Git;
- no Oteryn staging/production secrets;
- no inbound exposure;
- bounded outbound connectivity only for the approved official component/session phase;
- deterministic cleanup of temporary profile/history state.

Historical WSL2/WSLg evidence proves the synthetic harness can exercise graphical lifecycle and network isolation. It does **not** prove the official client or BattlEye accepts virtualization. A refusal must not be worked around.

## Credential handling

The owner must never paste a password, authenticator code or reusable session material into ChatGPT, GitHub, task/PR/issue text, workflow input, command line or shell history.

Acceptable mechanisms, in preference order:

1. manual entry directly into the official client;
2. local interactive secret prompt using a private pipe/protected descriptor with no logging/retention;
3. owner-created temporary local secret-store entry that is deleted/revoked after use.

The harness must fail closed if secret-like material appears in configuration, process arguments, retained outputs or tracked files.

## Preserved harness architecture

The existing PR #391 harness is preserved unchanged by Issue #886.

### Preflight

- verifies Linux/display/runtime conditions;
- verifies exact package/executable hash and ELF Build ID before official launch;
- requires encrypted/private evidence storage outside Git;
- rejects tracing/injection indicators and unsafe secret exposure;
- records only redacted preflight metadata.

### Launcher

- launches the exact unmodified client or deterministic fake client;
- never modifies/preloads/debugs/injects into the official process;
- supports no-network fake-client dry run;
- official component mode forbids authentication and outbound connectivity until its gate is satisfied.

### Evidence / redaction / cleanup

Allowed evidence is bounded process/window/timing/endpoint metadata, local file inventory and owner-approved observations. Raw captures/screenshots, if later explicitly enabled, remain private.

Before anything leaves private storage, scan for credentials, account/character identifiers, session/token material, raw payloads and unnecessarily identifying local paths. Cleanup must prove temporary secrets/processes/files are gone and Git-visible outputs remain clean.

## Phase gates

### Phase 1 — synthetic/no-network harness

`PROVEN`

Historical branch evidence demonstrated a graphical fake client in a distinct loopback-only network namespace, denied reserved-address connection, synthetic secret scanning, manifest validation and deterministic cleanup without contacting the official service.

### Phase 2 — exact official component, no authentication

`BLOCKED`

Requires all of:

- provably encrypted private evidence storage;
- exact owner-approved official package path and expected identity;
- dedicated accepted research environment;
- unmodified official client/BattlEye launch without bypass;
- authentication disabled for this gate.

### Phase 3 — bounded owner-gated live observation

`BLOCKED`

Requires Phase 2 plus owner-supplied local account/character selection, secure manual/ephemeral credential entry and an exact minimal observation script. Stop on any anti-cheat/account-security warning or evidence-risk expansion.

### Phase 4 — analyze and hand off

Classify every result as `PROVEN`, `DERIVED`, `UNKNOWN` or `CONFLICT`.

Current handoff matrix:

| Observation domain | Current implementation/follow-up authority | Reference-only evidence |
|---|---|---|
| Identity/login/ticket/Gateway routing | Oteryn Platform | official client behavior, historical login/Canary paths |
| Native Rust client | Oteryn-v2 | historical `blakinio/otclient`, official client behavior |
| Native gameplay protocol | Oteryn-v2 | Canary/Tibia protocol observations |
| Native game/world state | Oteryn-v2 | Canary runtime/content observations |
| Legacy compatibility | separately authorized compatibility task | OTClient/Canary/Tibia evidence |

No external repository is modified by this task. A cross-repository implementation follow-up requires separate authorization in the owning repository.

## Evidence-to-Oteryn rules

- Official observations may reveal behavior to investigate; they do not define Oteryn-v2 architecture by themselves.
- Historical OTClient and Canary source may explain compatibility behavior; they do not supersede Oteryn-v2 native authority.
- Platform documents only Platform-owned Identity/Gateway/control-plane requirements and references native game/client authority rather than duplicating it.
- No raw official packet payload or protected implementation detail is required to state a safe behavioral gap.
- Unknown or unsafe-to-observe behavior remains `UNKNOWN`; do not fill it with assumptions.

## Deliverables retained on historical PR #391

- `tools/tibia-linux-reference/README.md`;
- bounded preflight/launcher/redaction/cleanup tooling;
- deterministic fake-client tests;
- redacted session-manifest schema/example;
- Linux workflow;
- this plan;
- `docs/agents/reports/OTERYN-20260801-official-linux-client-live-observation.md`.

The harness/workflow are not modified by Issue #886.

## Current readiness decision

`external_service_execution_ready = false`

Current decisive blockers remain:

- encrypted evidence storage is not proven;
- exact approved official package identity is unavailable;
- unmodified official component/BattlEye launch has not been proven;
- no current owner-gated live session is authorized for a specific native-v2 evidence gap.

Therefore no official-service authentication or live session is performed as part of the authority reconciliation.

## PR #391 disposition

PR #391 is a historical research branch whose unique harness remains recoverable from Git. It must not be merged from its old base merely to preserve that work.

After the Issue #886 documentation reconciliation, close PR #391 as superseded. If live-reference research is resumed later, open a fresh task/PR from then-current `main`, revalidate the harness pieces deliberately, and route native findings to Oteryn-v2 while keeping Platform findings in Oteryn-Platform.
