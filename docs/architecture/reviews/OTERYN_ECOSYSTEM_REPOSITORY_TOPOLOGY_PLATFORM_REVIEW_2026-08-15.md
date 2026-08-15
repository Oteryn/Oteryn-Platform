# Verdict

**ACCEPT_WITH_CHANGES**

ADR 0040 has the right high-level repository split and the right semantic ownership for Portal, Identity, Game Gateway, Game, and Atlas. The review found two material architecture/governance defects that should be corrected through a new superseding ecosystem ADR rather than by editing Accepted ADR 0040 in place:

1. the future transfer of ecosystem authority from `Oteryn-Platform` to the planned `Oteryn` meta repository is not defined strictly enough to prevent two source-of-truth copies of the same cross-repository decision;
2. the `/map` integration language permits models that can make Platform a build/release consumer or packaging owner of Atlas implementation assets, which would weaken the repository extraction boundary that ADR 0040 is intended to establish.

The topology itself should not be rejected. The correction should narrow authority, contract ownership, and deployment-integration invariants without moving runtime code or changing production behavior in this review.

# Facts

- **FACT:** Review baseline is `blakinio/Oteryn-Platform` `main` at `aaac24350aa60f610507792d737948abe8a30b50` on 2026-08-15.
- **FACT:** Platform architecture authority is governed by [`../ARCHITECTURE_AUTHORITY.md`](../ARCHITECTURE_AUTHORITY.md). Its hierarchy places Accepted ADRs above `SYSTEM_ARCHITECTURE`, `MODULE_CATALOG`, contracts, and supporting documentation. Material correction of an Accepted ADR requires a new ADR/supersession rather than an untracked rewrite.
- **FACT:** [`../SYSTEM_ARCHITECTURE.md`](../SYSTEM_ARCHITECTURE.md) defines Platform as the web/application/control-plane side of the ecosystem. It keeps gameplay, world, and native game-session authority outside Platform while keeping account, entitlement, authentication gating, and canonical Platform-to-game identity mapping inside Platform.
- **FACT:** [`../MODULE_CATALOG.md`](../MODULE_CATALOG.md) assigns `PublicPortal`, `Identity`, `Accounts`, and `Integration` to Platform. It also lists `GameGateway` as a Platform host/deployable rather than a native-game subsystem.
- **FACT:** [`../PLATFORM_API_ARCHITECTURE.md`](../PLATFORM_API_ARCHITECTURE.md) places API authentication context under Platform Identity and exposes Platform-owned integration surfaces such as status and GameAuth.
- **FACT:** Accepted [`../adr/0031-native-oteryn-v2-integration-boundary.md`](../adr/0031-native-oteryn-v2-integration-boundary.md) assigns web/account/auth/operational control to Platform and gameplay/world/native-session authority to the native game side. It makes GameAuth the primary account-scoped integration boundary and requires versioned/backward-compatible protocol evolution.
- **FACT:** Accepted [`../adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md`](../adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md) selects four target repositories: `Oteryn` meta, `Oteryn-Game`, `Oteryn-Platform`, and `Oteryn-Atlas`. It keeps canonical world/content and import tooling on the Game side, keeps Portal/Identity/Game Gateway on Platform, and gives Atlas ownership of the browser map product and Game-derived map data.
- **FACT:** The current Game Gateway has no separate authoritative top-level architecture document named `GAME_GATEWAY_ARCHITECTURE.md` on the reviewed `main`. Its architectural role is defined by `SYSTEM_ARCHITECTURE`, `MODULE_CATALOG`, integration architecture, and contracts, especially [`../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md`](../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md).
- **FACT:** [`../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md`](../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md) makes Gateway a bridge to authoritative Platform identity and explicitly prevents it from becoming an alternate identity store or owner of duplicated game/account state.
- **FACT:** [`../../contracts/AUTH_GAME_LOGIN_CONTRACT.md`](../../contracts/AUTH_GAME_LOGIN_CONTRACT.md), [`../../contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md`](../../contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md), and [`../OTERYN_V2_INTEGRATION_ARCHITECTURE.md`](../OTERYN_V2_INTEGRATION_ARCHITECTURE.md) preserve the Platform-to-native trust boundary: Platform issues/coordinates bounded identity/session claims; native Game validates admission and remains authoritative for gameplay/runtime session semantics after admission.
- **FACT:** Closed PR #1065 (`docs(architecture): define public Map atlas integration`) was a draft, was not merged, and was explicitly closed on 2026-08-15 as superseded historical evidence. Its proposed ADR 0038 and `PUBLIC_MAP_ATLAS_CONTRACT.md` never became architecture authority. Its closeout records that the legacy `blakinio/Otheryn` Atlas premise was invalidated in favor of a separate future `Oteryn-Atlas` and a separate canonical Game/world owner.
- **FACT:** PR #1065 is useful historical evidence because it demonstrates a concrete coupling risk: the draft model assigned Platform deployment-time acquisition, verification, same-origin publication, cache/security policy, and activation of Atlas artifacts. Those choices were never accepted and must not be carried forward merely because `/map` remains desirable.
- **FACT:** This review changes documentation only. It does not authorize or modify runtime, deployment, Synology, production, DNS, authentication behavior, or an external repository.

# Agreements

1. **Portal should remain a Platform module.**
   - **FACT:** PublicPortal is already the Platform presentation/composition surface for player-facing web and APIs.
   - **INFERENCE:** Moving Portal out would split ordinary web/application composition from the business/application modules it presents without creating a new independent domain authority.
   - **RECOMMENDATION:** Keep Portal in `Oteryn-Platform`; allow it to link to or route users toward independently deployed products without absorbing their implementation.

2. **Identity should remain a Platform module.**
   - **FACT:** Identity participates directly in Accounts, Entitlements, API authentication, GameAuth, and Gateway contracts.
   - **INFERENCE:** A repository split now would turn existing in-process/module contracts into distributed trust, availability, PII, and versioning boundaries without evidence of an independently owned identity product.
   - **RECOMMENDATION:** Keep Identity in `Oteryn-Platform` unless future operational evidence satisfies explicit extraction criteria.

3. **Game Gateway should remain in the Platform repository even though it is a separate deployable.**
   - **FACT:** A deployment boundary already exists without a repository boundary. Gateway consumes Platform Identity/Integration/GameAuth semantics and is forbidden from owning alternate identity or game-world truth.
   - **RECOMMENDATION:** Preserve the separate executable/deployment lifecycle while keeping source and semantic authority in Platform.

4. **There is no current architecture evidence requiring a dedicated Gateway or Identity repository.**
   - **RECOMMENDATION:** Consider extraction only after at least several of these become true: independent long-lived ownership/team, materially independent release cadence, stable network API replacing module coupling, independent security/compliance lifecycle, multiple non-Platform consumers, distinct data authority, or materially different availability/scaling requirements that cannot be isolated as deployables inside the current repository.

5. **Canonical world/content, OTBM import, and Game-derived source truth belong with Game; Atlas should consume exports and never become the canonical world editor/source.**

6. **The browser map product should be an Atlas responsibility.** Platform should own discovery/integration policy, not map viewer/search/layer/render implementation.

7. **The future `Oteryn` meta repository is the right place for ecosystem-level compatibility/release composition and cross-repository E2E coordination.** It must not become a dumping ground for provider-owned API schemas or Platform-local architecture.

# Concerns

1. **Authority transfer after meta-repository creation is underspecified.**
   - **CONCERN:** ADR 0040's transitional placement in Platform is reasonable, but wording around a future canonical copy/top-level decision can be implemented as duplication. A copied Accepted ADR in two repositories creates ambiguous precedence, review ownership, and amendment history.
   - **IMPACT:** Cross-repository agents and maintainers could resolve the same question from different authoritative-looking documents.

2. **ADR 0040 is broader than Platform's permanent architecture authority.**
   - **FACT:** It allocates ownership across four repositories and defines responsibilities for Game and Atlas, neither of which is a Platform-local module.
   - **INFERENCE:** It is acceptable in Platform only as transitional ecosystem authority while no meta repository exists. It is not an appropriate permanent home for the ecosystem topology.

3. **The `/map` integration alternatives are too permissive.**
   - **CONCERN:** “Platform route + Atlas assets”, package/build integration, or Platform-side artifact acquisition can silently make Platform responsible for Atlas packaging, asset activation, or release coordination.
   - **IMPACT:** Atlas can become logically separate but operationally coupled to every Platform release, recreating the coupling that extraction is meant to remove.

4. **Same-origin integration can leak Platform credentials unless the trust boundary is explicit.**
   - **CONCERN:** A reverse proxy under the Platform public origin can forward ambient cookies, `Authorization`, CSRF headers, tracing baggage, or other Platform request context to Atlas by default.
   - **RECOMMENDATION:** Treat Atlas as an unprivileged public consumer by default. Strip Platform authentication/session material at the proxy boundary unless a separately approved authenticated contract requires a narrowly scoped credential.

5. **Cross-repository contract ownership is not yet explicit enough.**
   - **CONCERN:** A future meta repository can become a second copy of Platform API, Game protocol, or Atlas manifest schemas if “cross-repo contracts” is interpreted as “copy all shared contracts into meta”.
   - **RECOMMENDATION:** Meta should own contract discovery, compatibility sets, release manifests, and ecosystem invariants; each provider should remain canonical owner of its protocol/API/schema.

6. **Repository separation must not be confused with security isolation.** Separate repositories do not by themselves create better trust boundaries. Keys, session claims, token audiences, deployment identities, data authority, and network policies define the security boundary.

# Requested Changes To ADR 0040

Do **not** edit ADR 0040 in place. Under the current architecture authority, record these changes in a new ecosystem ADR that explicitly supersedes ADR 0040 after owner acceptance.

The superseding ADR should:

1. state that `Oteryn-Platform` ADR 0040 remains the historical decision record and is never copied as a second Accepted authority;
2. define the future `Oteryn` meta repository as the canonical owner of ecosystem topology, ecosystem compatibility/release composition, and cross-repository E2E orchestration;
3. use explicit metadata such as `Supersedes: blakinio/Oteryn-Platform ADR-0040` and a canonical repository/path identifier;
4. preserve `Oteryn-Platform` as canonical owner of Portal, Identity, Accounts, GameAuth, Gateway semantics, Platform API, and Platform-local threat/deployment architecture;
5. preserve `Oteryn-Game` as canonical owner of native Client/Server, `protocol-oteryn`, shared Game-side Rust/domain primitives, canonical world/content, OTBM import, and Studio;
6. preserve `Oteryn-Atlas` as canonical owner of the browser map product, viewer/search/layers/overlays, derived map data, Atlas release artifacts, and Atlas runtime/deployment packaging;
7. narrow `/map` to an integration invariant: Platform owns public discovery/URL policy; Atlas owns application/runtime/assets/release. The preferred same-origin form is an edge/reverse-proxy mount to an independently deployable Atlas, not copying Atlas assets into the Platform application artifact;
8. state that a dedicated Atlas subdomain is a valid future deployment choice if origin isolation, CDN/cache, scaling, or release independence outweigh the same-origin UX benefit;
9. define one-owner contract rules: provider schemas live with the provider; meta records compatibility and pins exact versions instead of copying schemas;
10. define extraction criteria for any future Identity/Gateway repository split instead of treating deployability as sufficient evidence for a repository split;
11. define the authority-transfer procedure and a prohibition on two normative copies of the same ecosystem ADR/contract.

**Proposed successor ADR title:** `Ecosystem Repository Authority, Cross-Repository Contracts, and Atlas Integration Boundary`.

**RECOMMENDATION:** Place that successor in the future `Oteryn` meta repository when it exists. Until then, ADR 0040 remains the Accepted transitional authority and this review is advisory evidence; it does not override the current hierarchy.

# Platform Ownership Matrix

| Capability / artifact | Canonical owner | Platform role | Repository-split conclusion |
|---|---|---|---|
| Public Portal shell, public web/API composition | `Oteryn-Platform` | Owner | Stay in Platform |
| Identity | `Oteryn-Platform` | Owner | Stay in Platform |
| Accounts / Entitlements | `Oteryn-Platform` | Owner | Stay in Platform |
| GameAuth | `Oteryn-Platform` | Owner | Stay in Platform |
| Game Gateway executable | `Oteryn-Platform` | Owner; separately deployable | Stay in Platform now |
| Platform API | `Oteryn-Platform` | Owner/provider | Stay in Platform |
| Native gameplay/world/session authority | `Oteryn-Game` | External consumer/integration peer | Must not move to Platform |
| `protocol-oteryn` and native protocol semantics | `Oteryn-Game` | Consumer/adapter where needed | Must not be duplicated in Platform |
| Canonical world/content | `Oteryn-Game` | Consumer only where business needs require | Must not move to Platform/Atlas |
| OTBM importer | `Oteryn-Game` | No implementation ownership | Must not move to Platform |
| Browser map viewer/search/layers/overlays | `Oteryn-Atlas` | Discovery/integration only | Must not be implemented by Platform |
| Derived map dataset and Atlas release | `Oteryn-Atlas`, derived from versioned Game export | Consumer/integration only | Must not be republished as Platform-owned source truth |
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
- ecosystem-wide dependency direction rules that prevent Platform, Game, or Atlas from becoming a second source of truth for another repository's domain.

# What Must Stay In Platform

- [`../SYSTEM_ARCHITECTURE.md`](../SYSTEM_ARCHITECTURE.md) for Platform internals and Platform boundaries;
- [`../MODULE_CATALOG.md`](../MODULE_CATALOG.md) for Platform module ownership;
- [`../PLATFORM_API_ARCHITECTURE.md`](../PLATFORM_API_ARCHITECTURE.md);
- Platform Identity/Accounts/Entitlements architecture and semantics;
- GameAuth/Gateway architecture, threat models, sequence diagrams, and Platform-owned parts of native integration;
- [`../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md`](../../contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md), [`../../contracts/AUTH_GAME_LOGIN_CONTRACT.md`](../../contracts/AUTH_GAME_LOGIN_CONTRACT.md), and other Platform-provider API/session contracts;
- Platform-side adapters and translation rules that do not redefine the native protocol authority;
- Platform security controls, auth/session/key handling, operational/deployment/runbook documentation, and Platform-local test policy;
- ADR 0031 as Platform architectural history/current Platform-native boundary until explicitly superseded;
- ADR 0040 as immutable historical/transitional record after supersession, with a pointer to the future canonical ecosystem ADR rather than a second normative copy;
- a small local ecosystem-authority index/pointer telling Platform contributors where current cross-repository authority resides.

# Cross-Repository Contracts Needed

## Platform <-> Atlas

The boundary should be explicit before runtime integration. The contract should cover:

- **URL/deep-link contract:** public entry URL, base path, SPA fallback behavior, supported query/deep-link state, and redirect compatibility;
- **release identity:** Atlas application/build ID, release-manifest schema version, immutable artifact digests, and rollback identity;
- **Game provenance:** exact Game release/export ID, export schema version, canonical world/content build identity, and source digest used by each Atlas release;
- **compatibility:** supported version ranges and a machine-readable compatibility result that can be pinned by the meta release manifest;
- **failure behavior:** unavailable/stale/incompatible Atlas behavior without making Portal unavailable;
- **cache contract:** immutable content URLs, active-version metadata semantics if used, and no mixed-version activation;
- **security boundary:** CSP/CORS policy, cookie/header forwarding policy, no direct Platform database access, no ambient Platform credentials by default, and no Atlas authority over Identity/Accounts/GameAuth;
- **authenticated future features:** only through a stable Platform API with explicit scopes/audience and least privilege; do not grant Atlas module/database coupling merely because it is served under the same origin;
- **telemetry:** correlation/error observability without copying credentials or unnecessary PII across the boundary.

**Recommended deployment interpretation for `/map`:** keep `/map` as the user-facing Platform ecosystem URL, but route `/map` and Atlas-owned static/application paths at the edge/reverse proxy to an independently built and deployable Atlas. Platform Portal owns the navigation target and public integration policy; Atlas owns the bytes and runtime. A Platform package/build should not normally vendor the Atlas application.

A dedicated Atlas subdomain remains a valid alternative when stronger origin/cache/scaling/release isolation is required. If selected later, Platform should redirect/link there and the contract must add CORS/CSP/cookie/deep-link rules. An iframe is not recommended as the default composition model.

## Platform / Gateway <-> Oteryn-Game

Existing Platform contracts already cover substantial parts of this boundary. The cross-repository contract set should be made explicitly versioned and should include:

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
- cross-repository E2E scenarios proving account identity, successful/failed admission, expiry, key rotation, incompatible version rejection, and session termination behavior.

Canonical API/claim schemas should remain with the repository that provides/owns them. Meta should point to and pin those artifacts rather than fork their text.

# Security / Deployment Implications

- **FACT:** This review performs no deployment or security-control mutation.
- **RECOMMENDATION:** Same-origin `/map` should be treated as a routing convenience, not as evidence that Atlas belongs to Platform's trust boundary.
- **RECOMMENDATION:** A same-origin reverse proxy must deny/strip Platform session cookies, `Authorization`, CSRF material, internal headers, and privileged service credentials toward Atlas by default. Only an explicitly reviewed allowlist may cross the boundary.
- **RECOMMENDATION:** Atlas should not receive direct access to Platform databases, Identity stores, GameAuth signing material, or Gateway internal credentials.
- **RECOMMENDATION:** If Atlas later needs user-specific features, use explicit Platform API contracts and purpose-scoped tokens/audiences rather than ambient cookie forwarding.
- **RECOMMENDATION:** A subdomain offers stronger browser-origin separation and simpler independent CDN/cache policy but introduces explicit CORS/CSP/cookie/deep-link and observability work. This is a deployment choice, not a repository-ownership choice.
- **RECOMMENDATION:** GameAuth and Gateway signing/verification boundaries should remain deployable independently from Platform Portal, but repository extraction is not required to achieve process/network isolation.
- **INFERENCE:** Splitting Identity into a separate repository/service today would increase distributed-auth failure modes and operational trust surface more than it would reduce architectural coupling, because the current contracts show Identity as a Platform authority used by several Platform modules.

# Migration Risks

## P0

- **None identified by this documentation architecture review.** This is not a production penetration test or runtime security audit, so absence of a P0 here is not evidence that no runtime P0 exists.

## P1

- two Accepted-looking copies of ecosystem authority after meta creation;
- Platform credentials/session context accidentally forwarded to an independently trusted Atlas under same-origin proxying;
- independent Platform/Game releases breaking GameAuth/admission compatibility without a pinned compatibility set;
- Atlas publishing derived data without immutable provenance back to a specific canonical Game export/release;
- Atlas or Platform becoming a second canonical source for world/content data.

## P2

- Platform vendoring/bundling Atlas artifacts and thereby coupling Atlas release cadence to Platform builds;
- `/map` base-path, deep-link, cache, or SPA-fallback incompatibility across independent Atlas releases;
- drift between provider-owned contracts and copied meta/consumer documents;
- Gateway/native version skew not represented in the ecosystem release manifest;
- migration of cross-repo documents without clear owner/path metadata, leaving dead or misleading references.

## P3

- route/name/link churn during a future same-origin-to-subdomain transition;
- stale non-authoritative historical references being mistaken for current design;
- documentation/index maintenance friction when repositories are created or renamed.

# Open Decisions

1. Exact `/map` serving topology after `Oteryn-Atlas` exists: same-origin edge/reverse-proxy mount versus dedicated Atlas subdomain. This review recommends the same-origin proxy model as the default user-facing integration if it can preserve credential isolation and independent Atlas deployment.
2. Exact machine-readable Atlas release/provenance manifest schema and its canonical repository path.
3. Exact Game world/content export schema presented to Atlas and the compatibility/version policy for that export.
4. Exact location/format of the future meta compatibility manifest and contract registry.
5. Whether the future meta repository owns only orchestration/configuration or also a thin executable E2E harness; it should not absorb product runtime.
6. Formal quantitative/operational thresholds that would justify extracting Identity or Gateway into independent repositories in the future.
7. Whether a dedicated follow-up Platform ADR is needed for `/map` routing/security once actual Atlas deployment constraints are known. Such an ADR should decide Platform integration mechanics only, not Atlas internals.

# Final Recommendation

Keep the four-repository target topology from ADR 0040, with `Oteryn-Platform` retaining Portal, Identity, Accounts, GameAuth, and Game Gateway. Do not create separate Gateway or Identity repositories now.

Treat `Oteryn-Atlas` as an independently built/deployed browser product whose derived data is provably tied to immutable `Oteryn-Game` exports. Platform may own the user-facing `/map` discovery/URL policy, but it should not own the Atlas build, runtime, derived-data pipeline, or release payload. Prefer a same-origin edge/reverse-proxy mount for `/map` when it can preserve independent Atlas deployment and strip Platform credentials; use a dedicated Atlas subdomain if stronger origin/deployment isolation is the better trade-off.

The review identifies a real governance/design correction requirement in ADR 0040. Therefore do not amend ADR 0040 silently. When the future `Oteryn` meta repository exists, create and accept a new ecosystem ADR — proposed title `Ecosystem Repository Authority, Cross-Repository Contracts, and Atlas Integration Boundary` — with explicit `Supersedes: blakinio/Oteryn-Platform ADR-0040`. Keep ADR 0040 in Platform as immutable historical/transitional evidence and replace its future authority with a pointer to the successor.

Until that superseding ADR is accepted, ADR 0040 remains the current Accepted authority under `ARCHITECTURE_AUTHORITY.md`; this review is a formal change request, not a second source of architecture truth.