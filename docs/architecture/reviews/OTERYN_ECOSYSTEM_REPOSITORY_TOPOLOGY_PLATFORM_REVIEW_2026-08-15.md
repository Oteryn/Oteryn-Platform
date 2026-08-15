# Verdict

**ACCEPT_WITH_CHANGES**

ADR 0040 has the right high-level repository split and the right semantic ownership for Portal, Identity, Game Gateway, Game, and Atlas. A second senior engineering / programming / project-delivery review confirms that the topology should be kept, but it identifies material corrections that should be captured by a future superseding ecosystem ADR rather than by editing Accepted ADR 0040 in place:

1. the future transfer of ecosystem authority from `Oteryn-Platform` to the planned `Oteryn` meta repository is not defined strictly enough to prevent two source-of-truth copies of the same cross-repository decision;
2. the `/map` integration language must preserve Atlas release independence and must not make Platform a build/package owner of Atlas implementation assets;
3. an independently released Atlas must not be treated as an unprivileged application while executing JavaScript on the authenticated Platform browser origin; a separate browser origin is the safer default unless Atlas is deliberately accepted into the full Platform application-security trust boundary.

The topology itself should not be rejected. The correction should narrow authority, contract ownership, browser-origin trust, release coupling, and migration sequencing without moving runtime code or changing production behavior in this review.

# Facts

- **FACT:** Initial review baseline was `blakinio/Oteryn-Platform` `main` at `aaac24350aa60f610507792d737948abe8a30b50` on 2026-08-15.
- **FACT:** Second-pass review reconciled current `main` at `5847973676ba82b74aaac7d5cc90238c262dd541`. The only intervening main change is merged PR #1099, which archives the already completed portal-programme task; it does not change the architecture sources reviewed here.
- **FACT:** Platform architecture authority is governed by [`../ARCHITECTURE_AUTHORITY.md`](../ARCHITECTURE_AUTHORITY.md). Its hierarchy places Accepted ADRs above focused architecture, while unresolved durable architecture obligations are routed through `ARCHITECTURE_DECISION_BACKLOG.json`. Material correction of an Accepted ADR requires a new ADR/supersession rather than an untracked rewrite.
- **FACT:** [`../SYSTEM_ARCHITECTURE.md`](../SYSTEM_ARCHITECTURE.md) defines Platform as the web/application/control-plane side of the ecosystem. It keeps gameplay, world, and native game-session authority outside Platform while keeping account, entitlement, authentication gating, and canonical Platform-to-game identity mapping inside Platform.
- **FACT:** [`../MODULE_CATALOG.md`](../MODULE_CATALOG.md) assigns `PublicPortal`, `Identity`, `Accounts`, and `Integration` to Platform. It also lists `GameGateway` as a Platform host/deployable rather than a native-game subsystem.
- **FACT:** [`../PLATFORM_API_ARCHITECTURE.md`](../PLATFORM_API_ARCHITECTURE.md) explicitly defers the general PlatformAPI until a named consumer/use case exists. Existing GameAuth endpoints, internal Gateway transports, and operational probes are specialized bounded contracts and must not be reclassified as activation or completeness of the general PlatformAPI.
- **FACT:** Accepted [`../adr/0031-native-oteryn-v2-integration-boundary.md`](../adr/0031-native-oteryn-v2-integration-boundary.md) assigns web/account/auth/operational control to Platform and gameplay/world/native-session authority to the native game side. It makes GameAuth the primary account-scoped integration boundary and requires versioned/backward-compatible protocol evolution.
- **FACT:** Accepted [`../adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md`](../adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md) selects four target repositories: `Oteryn` meta, `Oteryn-Game`, `Oteryn-Platform`, and `Oteryn-Atlas`. It keeps canonical world/content and import tooling on the Game side, keeps Portal/Identity/Game Gateway on Platform, and gives Atlas ownership of the browser map product and Game-derived map data.
- **FACT:** The current Game Gateway has no separate authoritative top-level architecture document named `GAME_GATEWAY_ARCHITECTURE.md` on the reviewed `main`. Its architectural role is defined by `SYSTEM_ARCHITECTURE`, `MODULE_CATALOG`, integration architecture, and contracts, especially [`../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md`](../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md).
- **FACT:** [`../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md`](../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md) makes Gateway a bridge to authoritative Platform identity and explicitly prevents it from becoming an alternate identity store or owner of duplicated game/account state.
- **FACT:** [`../../contracts/AUTH_GAME_LOGIN_CONTRACT.md`](../../contracts/AUTH_GAME_LOGIN_CONTRACT.md), [`../../contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md`](../../contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md), and [`../OTERYN_V2_INTEGRATION_ARCHITECTURE.md`](../OTERYN_V2_INTEGRATION_ARCHITECTURE.md) preserve the Platform-to-native trust boundary: Platform issues/coordinates bounded identity/session claims; native Game validates admission and remains authoritative for gameplay/runtime session semantics after admission.
- **FACT:** [`../SECURITY_ARCHITECTURE.md`](../SECURITY_ARCHITECTURE.md) requires explicit trust boundaries, least privilege, one authoritative identity policy, appropriate session-cookie behavior, and same-origin browser script/connect restrictions for the Platform application.
- **FACT:** Closed PR #1065 (`docs(architecture): define public Map atlas integration`) was a draft, was not merged, and was explicitly closed on 2026-08-15 as superseded historical evidence. Its proposed ADR 0038 and `PUBLIC_MAP_ATLAS_CONTRACT.md` never became architecture authority.
- **FACT:** PR #1065 is useful historical evidence because it demonstrates a concrete coupling risk: the draft model assigned Platform deployment-time acquisition, verification, same-origin publication, cache/security policy, and activation of Atlas artifacts. Those choices were never accepted and must not be carried forward merely because `/map` remains desirable.
- **FACT:** PR #1100 received two material P2 review findings on its first review revision. One corrected an inaccurate description of the deferred general PlatformAPI boundary. The other correctly identified that reverse-proxy header stripping alone cannot make independently released same-origin Atlas JavaScript unprivileged, because same-origin script can directly call Platform paths with ambient browser credentials.
- **FACT:** This review changes documentation only. It does not authorize or modify runtime, deployment, Synology, production, DNS, authentication behavior, or an external repository.

# Agreements

1. **Portal should remain a Platform module.**
   - **FACT:** PublicPortal is already the Platform presentation/composition surface for player-facing web and business/application modules.
   - **INFERENCE:** Moving Portal out would split ordinary web/application composition from the business/application modules it presents without creating a new independent domain authority.
   - **RECOMMENDATION:** Keep Portal in `Oteryn-Platform`; allow it to discover/link/redirect to independently deployed products without absorbing their implementation.

2. **Identity should remain a Platform module.**
   - **FACT:** Identity participates directly in Accounts, Entitlements, session/authentication policy, GameAuth, and Gateway contracts.
   - **INFERENCE:** A repository split now would turn existing cohesive module contracts into distributed trust, availability, PII, and versioning boundaries without evidence of an independently owned identity product.
   - **RECOMMENDATION:** Keep Identity in `Oteryn-Platform` unless future operational evidence satisfies explicit extraction criteria.

3. **Game Gateway should remain in the Platform repository even though it is a separate deployable.**
   - **FACT:** A process/deployment boundary already exists without a repository boundary. Gateway consumes Platform Identity/Integration/GameAuth semantics and is forbidden from owning alternate identity or game-world truth.
   - **RECOMMENDATION:** Preserve the separate executable/deployment lifecycle while keeping source and semantic authority in Platform.

4. **There is no current architecture evidence requiring a dedicated Gateway or Identity repository.**
   - **RECOMMENDATION:** Consider extraction only after several independent signals become true: durable independent ownership/team, materially independent release cadence, a stable network contract replacing substantial in-repository coupling, independent security/compliance lifecycle, multiple non-Platform consumers, distinct data authority, or materially different availability/scaling requirements that cannot be isolated as deployables inside the current repository.

5. **Canonical world/content, OTBM import, and Game-derived source truth belong with Game; Atlas should consume explicit exports and never become the canonical world editor/source.**

6. **The browser map product should be an Atlas responsibility.** Platform should own discovery/integration policy, not map viewer/search/layer/render implementation or Atlas release packaging.

7. **The future `Oteryn` meta repository is the right place for ecosystem-level compatibility/release composition and cross-repository E2E coordination.** It must not become a dumping ground for provider-owned API schemas or Platform-local architecture.

8. **Repository boundaries and deployable boundaries are intentionally different concepts.** Independent scaling/restart/deployment is not sufficient evidence for an independent source repository.

9. **Release trains should remain loosely coupled.** A Portal release should not normally be blocked on rebuilding Atlas, and an Atlas release should not require rebuilding Platform merely to publish map viewer/data changes.

# Concerns

1. **Authority transfer after meta-repository creation is underspecified.**
   - **CONCERN:** ADR 0040's transitional placement in Platform is reasonable, but wording around a future canonical copy/top-level decision can be implemented as duplication. A copied Accepted ADR in two repositories creates ambiguous precedence, review ownership, and amendment history.
   - **IMPACT:** Cross-repository agents and maintainers could resolve the same question from different authoritative-looking documents.

2. **ADR 0040 is broader than Platform's permanent architecture authority.**
   - **FACT:** It allocates ownership across four repositories and defines responsibilities for Game and Atlas, neither of which is a Platform-local module.
   - **INFERENCE:** It is acceptable in Platform only as transitional ecosystem authority while no meta repository exists. It is not an appropriate permanent home for the ecosystem topology.

3. **The `/map` integration alternatives are too permissive if they are interpreted as build/package ownership.**
   - **CONCERN:** “Platform route + Atlas assets”, package/build integration, or Platform-side artifact acquisition can silently make Platform responsible for Atlas packaging, activation, rollback, or release coordination.
   - **IMPACT:** Atlas can become logically separate but operationally coupled to every Platform release, recreating the coupling that extraction is meant to remove.

4. **Same-origin Atlas JavaScript is a full browser-origin trust decision, not merely a reverse-proxy decision.**
   - **FACT:** Stripping credentials from requests that the reverse proxy forwards to an Atlas upstream does not prevent Atlas JavaScript already executing under the Platform origin from issuing requests to Platform endpoints. The browser may attach Platform cookies automatically and same-origin policy allows the script to read same-origin responses.
   - **IMPACT:** An independently released Atlas can become equivalent to trusted Platform application code for authenticated browser sessions if hosted on the same origin.
   - **RECOMMENDATION:** Prefer a distinct browser origin/subdomain for independently released Atlas code. A same-origin mount is acceptable only if the owner intentionally treats Atlas JavaScript as fully trusted Platform-origin code and subjects it to the same application-security, dependency, CSP, release, incident-response, and review boundary.

5. **Cross-repository contract ownership is not yet explicit enough.**
   - **CONCERN:** A future meta repository can become a second copy of Platform API, Game protocol, or Atlas manifest schemas if “cross-repo contracts” is interpreted as “copy all shared contracts into meta”.
   - **RECOMMENDATION:** Meta should own contract discovery, compatibility sets, release manifests, and ecosystem invariants; each provider should remain canonical owner of its protocol/API/schema.

6. **General PlatformAPI and specialized integration transports must remain separate concepts.**
   - **CONCERN:** Treating GameAuth, internal Gateway, or operations endpoints as the “Platform API” would silently activate a general API product that ADR 0036 explicitly defers.
   - **RECOMMENDATION:** Atlas must use public data or an explicitly approved bounded contract. If it becomes the named consumer that justifies a general PlatformAPI, that activation must follow `PLATFORM_API_ARCHITECTURE.md`; it cannot be assumed by repository topology.

7. **Repository separation must not be confused with security isolation.** Separate repositories do not by themselves create better trust boundaries. Browser origin, keys, session claims, token audiences, deployment identities, data authority, and network policy define real security boundaries.

8. **The migration sequence can become an unnecessarily serial project plan.**
   - **CONCERN:** Treating GitHub organization creation, meta-repository creation, repository transfers, Atlas extraction, contract design, and Platform integration as one strictly serial chain increases critical-path risk.
   - **RECOMMENDATION:** Preserve architectural order but parallelize evidence work: authority/meta bootstrap and contract design can proceed while the separately authorized legacy Atlas audit classifies code/history. Actual source migration waits for ownership/contracts; discovery need not wait for every organizational move.

9. **Atlas failure must not become Portal failure.**
   - **RECOMMENDATION:** The Platform/Atlas contract should require failure isolation, truthful degraded behavior, and independent rollback so Atlas outage or incompatible release does not make Portal/Identity/GameAuth unavailable.

# Requested Changes To ADR 0040

Do **not** edit ADR 0040 in place. Under the current architecture authority, record these changes in a new ecosystem ADR that explicitly supersedes ADR 0040 after owner acceptance.

The superseding ADR should:

1. state that `Oteryn-Platform` ADR 0040 remains the historical decision record and is never copied as a second Accepted authority;
2. define the future `Oteryn` meta repository as the canonical owner of ecosystem topology, ecosystem compatibility/release composition, and cross-repository E2E orchestration;
3. use explicit metadata such as `Supersedes: blakinio/Oteryn-Platform ADR-0040` and a canonical repository/path identifier;
4. preserve `Oteryn-Platform` as canonical owner of Portal, Identity, Accounts, GameAuth, Gateway semantics, Platform-local application/security architecture, and any future general PlatformAPI only after its separate activation criteria are met;
5. preserve `Oteryn-Game` as canonical owner of native Client/Server, `protocol-oteryn`, shared Game-side Rust/domain primitives, canonical world/content, OTBM import, and Studio;
6. preserve `Oteryn-Atlas` as canonical owner of the browser map product, viewer/search/layers/overlays, derived map data, Atlas release artifacts, and Atlas runtime/deployment packaging;
7. narrow `/map` to an integration invariant: Platform owns public discovery/entry policy; Atlas owns application/runtime/assets/release. A Platform build must not normally vendor an Atlas application release;
8. define browser-origin trust explicitly: an independently released Atlas should default to a distinct origin/subdomain. A same-origin deployment is permitted only as an explicit full-trust decision in which Atlas executable code is treated as Platform-origin application code and receives equivalent security/release governance;
9. allow `/map` to remain a stable Platform-facing alias/redirect/deep-link gateway to the independently hosted Atlas when a separate origin is selected;
10. define one-owner contract rules: provider schemas live with the provider; meta records compatibility and pins exact versions instead of copying schemas;
11. define extraction criteria for any future Identity/Gateway repository split instead of treating deployability as sufficient evidence for a repository split;
12. define the authority-transfer procedure and a prohibition on two normative copies of the same ecosystem ADR/contract;
13. define release independence and failure isolation: Platform and Atlas must be independently releasable/rollbackable, and Atlas unavailability must not take down Portal/Identity/GameAuth;
14. route the successor-ADR obligation through the architecture decision backlog while Platform remains the temporary cross-repository authority, then remove/transfer that obligation atomically when the meta authority becomes canonical.

**Proposed successor ADR title:** `Ecosystem Repository Authority, Cross-Repository Contracts, and Atlas Integration Boundary`.

**RECOMMENDATION:** Place the final ecosystem successor in the future `Oteryn` meta repository when that authority exists. Until then, ADR 0040 remains the Accepted transitional authority and this review is formal change-request evidence; it does not override the current hierarchy by itself.

# Platform Ownership Matrix

| Capability / artifact | Canonical owner | Platform role | Repository-split conclusion |
|---|---|---|---|
| Public Portal shell, public web composition | `Oteryn-Platform` | Owner | Stay in Platform |
| Identity | `Oteryn-Platform` | Owner | Stay in Platform |
| Accounts / Entitlements | `Oteryn-Platform` | Owner | Stay in Platform |
| GameAuth specialized transport/semantics | `Oteryn-Platform` | Owner | Stay in Platform |
| Game Gateway executable | `Oteryn-Platform` | Owner; separately deployable | Stay in Platform now |
| General PlatformAPI | `Oteryn-Platform` if/when activated | Currently deferred under ADR 0036 | Do not treat existing specialized transports as activation |
| Native gameplay/world/session authority | `Oteryn-Game` | External integration peer | Must not move to Platform |
| `protocol-oteryn` and native protocol semantics | `Oteryn-Game` | Consumer/adapter where needed | Must not be duplicated in Platform |
| Canonical world/content | `Oteryn-Game` | Consumer only where business needs require | Must not move to Platform/Atlas |
| OTBM importer | `Oteryn-Game` | No implementation ownership | Must not move to Platform |
| Browser map viewer/search/layers/overlays | `Oteryn-Atlas` | Discovery/integration only | Must not be implemented by Platform |
| Derived map dataset and Atlas release | `Oteryn-Atlas`, derived from versioned Game export | Consumer/integration only | Must not be republished as Platform-owned source truth |
| Atlas browser origin | `Oteryn-Atlas` deployment/security contract | Platform may provide alias/redirect | Prefer separate origin for independent trust/release |
| Ecosystem topology | future `Oteryn` meta | Transitional ADR currently in Platform | Move authority, not a duplicated copy |
| Cross-repo compatibility/release set | future `Oteryn` meta | Participant | Meta-owned |
| Cross-repo E2E orchestration | future `Oteryn` meta | Participant | Meta-owned; repo-local tests remain local |

# What Must Move To Future Meta

“Move” means transfer **authority**, normally by a new canonical artifact/superseding ADR and pointers from old locations; it does not mean copying normative documents and leaving both authoritative.

- canonical ecosystem repository topology and repository ownership map;
- the successor to ADR 0040;
- ecosystem compatibility matrix/version-set rules;
- release manifests that pin exact compatible Platform, Game, Atlas, and protocol versions/build IDs;
- cross-repository release-train policy and deprecation windows;
- cross-repository E2E orchestration and ecosystem acceptance gates;
- a canonical contract registry/index that points to the authoritative provider-owned contract/schema in each repository;
- Game -> Atlas provenance/compatibility invariants at ecosystem level: Game release/export identity, export schema version, Atlas build identity, and immutable digests;
- cross-repository migration and supersession rules;
- ecosystem-wide dependency direction rules that prevent Platform, Game, or Atlas from becoming a second source of truth for another repository's domain;
- ecosystem release policy stating which dependencies are hard compatibility gates versus independently deployable/soft dependencies.

# What Must Stay In Platform

- [`../SYSTEM_ARCHITECTURE.md`](../SYSTEM_ARCHITECTURE.md) for Platform internals and Platform boundaries;
- [`../MODULE_CATALOG.md`](../MODULE_CATALOG.md) for Platform module ownership;
- [`../PLATFORM_API_ARCHITECTURE.md`](../PLATFORM_API_ARCHITECTURE.md) and ADR 0036 for the intentionally deferred general PlatformAPI activation boundary;
- Platform Identity/Accounts/Entitlements architecture and semantics;
- GameAuth/Gateway architecture, threat models, sequence diagrams, and Platform-owned parts of native integration;
- [`../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md`](../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md), [`../../contracts/AUTH_GAME_LOGIN_CONTRACT.md`](../../contracts/AUTH_GAME_LOGIN_CONTRACT.md), and other Platform-provider specialized API/session contracts;
- Platform-side adapters and translation rules that do not redefine native protocol authority;
- Platform security controls, auth/session/key handling, operational/deployment/runbook documentation, and Platform-local test policy;
- ADR 0031 as Platform architectural history/current Platform-native boundary until explicitly superseded;
- ADR 0040 as immutable historical/transitional record after supersession, with a pointer to the future canonical ecosystem ADR rather than a second normative copy;
- a small local ecosystem-authority index/pointer telling Platform contributors where current cross-repository authority resides.

# Cross-Repository Contracts Needed

## Platform <-> Atlas

The boundary should be explicit before runtime integration. The contract should cover:

- **URL/deep-link contract:** stable public entry URL, redirect/alias behavior, Atlas origin/base URL, supported deep-link state, and compatibility across origin/path transitions;
- **release identity:** Atlas application/build ID, release-manifest schema version, immutable artifact digests, and rollback identity;
- **Game provenance:** exact Game release/export ID, export schema version, canonical world/content build identity, and source digest used by each Atlas release;
- **compatibility:** supported version ranges and a machine-readable compatibility result that can be pinned by the meta release manifest;
- **release independence:** Platform does not need to rebuild merely for an Atlas release, and Atlas does not need a Platform application build merely to publish map changes;
- **failure behavior:** unavailable/stale/incompatible Atlas must degrade only the map capability and must not make Portal/Identity/GameAuth unavailable;
- **cache contract:** immutable content URLs, active-version metadata semantics if used, and no mixed-version activation;
- **browser-origin trust:** default separate origin for independently released Atlas executable code; if same-origin is intentionally selected, Atlas is treated as fully trusted Platform-origin application code, not as an unprivileged proxied consumer;
- **cookie/session policy:** Platform session cookies should not be broadly scoped to an Atlas origin; host/domain/path attributes and credentialed requests must be explicit and tested;
- **CSP/CORS policy:** exact allowed origins/resource classes, with no accidental broadening merely to make integration convenient;
- **authorization boundary:** no direct Platform database access, no GameAuth signing material, and no ambient privileged service credentials;
- **authenticated future features:** use an explicit bounded Platform-owned contract with purpose-scoped authorization. If Atlas is the named consumer that triggers a general PlatformAPI, follow `PLATFORM_API_ARCHITECTURE.md`; do not treat GameAuth/internal Gateway routes as the general API;
- **telemetry:** correlation/error observability without copying credentials or unnecessary PII across the boundary.

**Recommended browser/deployment interpretation for `/map`:** keep `/map` as a stable player-facing Platform discovery URL, but default the independently released Atlas application to a distinct browser origin/subdomain. `/map` may redirect or otherwise resolve deep links to that origin. A same-origin edge/reverse-proxy mount is a possible future deployment only if Atlas executable code is intentionally accepted into the same browser-origin trust boundary as Platform itself.

An iframe is not recommended as the normal product composition model. It adds state/navigation/accessibility complexity without solving repository ownership and should be considered only for a separately justified isolation/use case.

## Platform / Gateway <-> Oteryn-Game

Existing Platform contracts already cover substantial parts of this boundary. The cross-repository contract set should be explicitly versioned and include:

- canonical Platform game-account identity semantics and mapping;
- GameAuth request/result/session claims and claim audience;
- key IDs, signature algorithms, verification, rotation/overlap, and revocation behavior;
- TTL, clock-skew, replay, nonce/idempotency, and bounded retry semantics;
- entitlement/version/admission requirements and stable failure codes;
- session sign/verify/end responsibility and ownership transitions;
- pre-admission handoff, with native Game becoming authoritative for admission/gameplay/session runtime after the documented boundary;
- protocol/capability negotiation, compatibility range, deprecation windows, and unsupported-version behavior;
- timeout/failure isolation so Game unavailability does not corrupt Platform identity/account state;
- a machine-readable compatibility tuple pinned in the future meta release manifest;
- cross-repository E2E scenarios proving account identity, successful/failed admission, expiry, key rotation, incompatible-version rejection, and session termination behavior.

Canonical API/claim schemas should remain with the repository that provides/owns them. Meta should point to and pin those artifacts rather than fork their text.

# Security / Deployment Implications

- **FACT:** This review performs no deployment or security-control mutation.
- **RECOMMENDATION:** Repository independence, deployment independence, and browser-origin isolation must be decided separately; none implies the others automatically.
- **RECOMMENDATION:** For independently released Atlas JavaScript, prefer a distinct browser origin/subdomain. This keeps ordinary Platform authenticated-origin privileges out of the Atlas trust boundary by default.
- **RECOMMENDATION:** If a same-origin Atlas mount is ever selected, treat Atlas JavaScript as Platform-trusted code and apply the same dependency review, CSP discipline, vulnerability response, release gate, and authenticated-browser regression testing as Platform code. Proxy header stripping alone is insufficient isolation.
- **RECOMMENDATION:** Atlas should not receive direct access to Platform databases, Identity stores, GameAuth signing material, Gateway internal credentials, or privileged service identities.
- **RECOMMENDATION:** If Atlas later needs user-specific features, use an explicit bounded contract and purpose-scoped authorization rather than ambient cookie forwarding. General PlatformAPI activation remains subject to ADR 0036.
- **RECOMMENDATION:** Atlas and Portal need independent health/failure domains. Routing integration must not turn an Atlas outage into a Platform login/portal outage.
- **RECOMMENDATION:** GameAuth and Gateway signing/verification boundaries should remain deployable independently from Platform Portal, but repository extraction is not required to achieve process/network isolation.
- **INFERENCE:** Splitting Identity into a separate repository/service today would increase distributed-auth failure modes and operational trust surface more than it would reduce architectural coupling, because current contracts show Identity as a Platform authority used by several Platform modules.

# Migration Risks

## P0

- **None identified by this documentation architecture review.** This is not a production penetration test or runtime security audit, so absence of a P0 here is not evidence that no runtime P0 exists.

## P1

- two Accepted-looking copies of ecosystem authority after meta creation;
- independently released Atlas JavaScript executing on the authenticated Platform browser origin without being intentionally included in the full Platform application-security trust boundary;
- independent Platform/Game releases breaking GameAuth/admission compatibility without a pinned compatibility set;
- Atlas publishing derived data without immutable provenance back to a specific canonical Game export/release;
- Atlas or Platform becoming a second canonical source for world/content data.

## P2

- Platform vendoring/bundling Atlas artifacts and thereby coupling Atlas release cadence to Platform builds;
- `/map` deep-link/origin/cache/SPA-fallback incompatibility across independent Atlas releases;
- Atlas downtime propagating to Portal availability through an overly coupled routing/deployment design;
- drift between provider-owned contracts and copied meta/consumer documents;
- Gateway/native version skew not represented in the ecosystem release manifest;
- migration of cross-repo documents without clear owner/path metadata, leaving dead or misleading references;
- treating GameAuth or operational endpoints as activation of the deferred general PlatformAPI;
- unnecessarily serial project sequencing that blocks evidence gathering on organization/repository administrative work.

## P3

- route/name/link churn during a future Atlas-origin transition;
- stale non-authoritative historical references being mistaken for current design;
- documentation/index maintenance friction when repositories are created or renamed.

# Open Decisions

1. Exact Atlas browser-origin and public URL topology after `Oteryn-Atlas` exists. This review now recommends a distinct Atlas origin/subdomain for independently released executable code, with `/map` retained as a stable Platform discovery/redirect alias. Same-origin remains an explicit full-trust alternative, not the default isolation model.
2. Exact machine-readable Atlas release/provenance manifest schema and its canonical repository path.
3. Exact Game world/content export schema presented to Atlas and the compatibility/version policy for that export.
4. Exact location/format of the future meta compatibility manifest and contract registry.
5. Whether the future meta repository owns only orchestration/configuration or also a thin executable E2E harness; it should not absorb product runtime.
6. Formal quantitative/operational thresholds that would justify extracting Identity or Gateway into independent repositories in the future.
7. Whether Atlas becomes a named consumer that activates a future general PlatformAPI or uses only public/bounded specialized contracts. Existing GameAuth/internal Gateway transports do not answer this question.
8. Exact project sequencing between meta-authority bootstrap, organization/repository administration, legacy Atlas audit, contract design, extraction, and integration. Architecture dependencies must be preserved without making independent discovery work artificially serial.
9. Whether a dedicated follow-up Platform ADR is needed for `/map` redirect/origin/security mechanics once actual Atlas deployment constraints are known. Such an ADR should decide Platform integration mechanics only, not Atlas internals.

# Final Recommendation

Keep the four-repository target topology from ADR 0040, with `Oteryn-Platform` retaining Portal, Identity, Accounts, GameAuth, and Game Gateway. Do not create separate Gateway or Identity repositories now.

Treat `Oteryn-Atlas` as an independently built/deployed browser product whose derived data is provably tied to immutable `Oteryn-Game` exports. Platform may own the user-facing `/map` discovery/entry policy, but it should not own the Atlas build, runtime, derived-data pipeline, or release payload. For independently released Atlas executable code, prefer a distinct browser origin/subdomain; keep `/map` as a stable Platform alias/redirect/deep-link entry. Use same-origin hosting only as an explicit decision to trust Atlas JavaScript as Platform-origin application code, not as a security-isolation shortcut.

Keep Platform/Atlas release trains independently deployable and failure-isolated. Do not make a Portal release rebuild Atlas, do not make an Atlas release rebuild Platform by default, and do not let Atlas unavailability take down Portal/Identity/GameAuth.

The review identifies a real governance/design correction requirement in ADR 0040. Therefore do not amend ADR 0040 silently. Route the successor obligation through the architecture decision backlog while Platform remains the temporary authority. When the future `Oteryn` meta repository exists, create and accept a new ecosystem ADR — proposed title `Ecosystem Repository Authority, Cross-Repository Contracts, and Atlas Integration Boundary` — with explicit `Supersedes: blakinio/Oteryn-Platform ADR-0040`. Keep ADR 0040 in Platform as immutable historical/transitional evidence and replace its future authority with a pointer to the successor.

Until that superseding ADR is accepted, ADR 0040 remains the current Accepted authority under `ARCHITECTURE_AUTHORITY.md`; this review is a formal change request and delivery-risk record, not a second source of architecture truth.
