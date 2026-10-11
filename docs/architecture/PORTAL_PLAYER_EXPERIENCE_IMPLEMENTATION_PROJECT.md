# Oteryn Portal player-experience implementation project

## Status, purpose and authority

Date: 2026-09-08. Documentation owner: Issue #1356 / PR #1357.

This is a **planning-only implementation specification**, prepared at the owner's request after the live Portal review and the recorded player-first direction. It is not a runtime delivery claim, a new programme, a scheduler, a feature activation decision or permission to operate another repository or environment.

Read alongside [the agreed direction](PORTAL_PLAYER_EXPERIENCE_COMPLETION_DIRECTION.md), [the accepted delivery plan](PORTAL_COMPLETION_DELIVERY_PLAN.md) and [the input register](PORTAL_PLAYER_EXPERIENCE_READINESS_INPUTS.json). `docs/agents/programs/OTERYN_PORTAL_COMPLETION.md` remains the sole live work selector. Proposed work packages below describe dependencies and acceptance **after selection**; they do not reorder that selector or promote conditional/deferred capabilities. `PORTAL-POLISH` remains terminal.

The present task only analyses permitted Platform sources and prepares documentation. Runtime implementation, publishing CMS records, activation, real account operations, client execution, protected environments and production deployment are not performed or newly authorized by this project. Existing protection, privacy, source ownership and release gates remain authoritative.

## 1. Evidence and the actual problem

Source inspection baseline: `Oteryn/Oteryn-Platform@2acb8b548b413b9ce522ff5c507b4f10c9d0ff0e`. Documentation continuation base: PR #1357 at `9d42ef34077a4c31460e1a43253a39ef97597bf1`. Sources below are relative to that inspected main, not assertions about a later release.

| ID | Classification | Verified source or bounded observation | Consequence for the project |
| --- | --- | --- | --- |
| E01 | FACT: source | `app/PublicPortal/HomePageQuery.php::worldSummary()` returns EMPTY when configured channels are empty; read failures become UNAVAILABLE and incomplete fresh channel runtime becomes STALE. | Diagnose the particular data/publication path. Do not infer service health or release stage from the displayed word EMPTY. |
| E02 | FACT: source | `app/PublicGameData/CanaryGameDataRepository.php::configuredChannels()` reads `channels` on the configured `canary` connection, filtered by `enabled = 1`. | An operational World Registry probe is not by itself evidence that this public query has a usable channel. Reconcile identifiers and source selection through accepted contracts, never by inserting invented rows. |
| E03 | FACT: source | `app/Downloads/PublicDownloadCenterQuery.php` requires a current published release, enabled artifacts and approved artifact URLs; optional platform filtering can leave no public artifact. | A missing download may be publication, eligibility, platform filtering or missing release evidence. It is not proof that a new download module must be built. |
| E04 | FACT: source | `app/Cms/Editorial/EditorialPageQuery.php` resolves a ManagedPage by its typed slug and distinguishes missing, unpublished, translation unavailable and published content. | A working getting-started route needs published content and valid localization, not just a styled template or a Markdown draft in Git. |
| E05 | FACT: source | `lang/pl/public.php` contains broken encoded Polish text and implementation-oriented onboarding descriptions at the inspected baseline. | A bounded localization/editorial repair is justified independently of the final art choice. Revalidate the source head before implementation. |
| E06 | FACT: source | `resources/views/game/layout.blade.php` loads shared styles plus a final owner-visible override and versions asset URLs by a 12-character content-hash prefix. | Keep content-based cache invalidation. Consolidate owning styles incrementally; do not require unchanged asset bytes to receive a different URL every release. |
| E07 | FACT: source | `scripts/acceptance/tests/portal-polish-quality.spec.mjs` checks HTTP, intrinsic image loading/sizing, overflow, asset versions, grid geometry and keyboard behaviour. Some assertions specifically fix the current 626 px artwork and column counts. | Preserve useful checks; evolve design-specific assertions with an approved design. These tests are not proof of published launch content, artwork authenticity or client/world-entry readiness. |
| E08 | FACT: source | `app/PublicPortal/HomepageTemplates/HomepageTemplateRegistry.php` contains code-owned production/classic views; `routes/modules/homepage-templates.php` protects preview/activation with authentication, MFA and `portal.settings.manage`. | Evaluate existing protected preview before proposing another preview system. It covers the homepage, not automatic rollback of every page family or CMS revision. |
| E09 | FACT: recorded earlier observation | The same conversation's 2026-09-08 host-browser review showed empty world/news states, no downloadable client, an unconfigured guide and Polish encoding problems. | Historical motivation only. No new live-browser session or deployed-SHA binding was performed for this project. Record fresh environment evidence in package PEX-01. |
| E10 | FACT: repository boundary | `THIRD_PARTY_NOTICES.md` explicitly does not establish rights for all maps, media, game data or generated/derived assets. | Resolve source and intended-publication rights for each chosen asset; unknown provenance is not a publication approval. |
| E11 | FACT: coordination | Issue #1115 is an open content-completion programme with source/profile/provenance boundaries. PR #1357 is the existing open documentation continuation. | Reuse content ownership rather than create a competing importer/programme; refresh its related PRs before any implementation assignment. |

**INFERENCE (high confidence):** the observed readiness gap has at least four independently testable dimensions: presentation, editorial publication, source/data availability and the real player-start path. A CSS-only task cannot establish all four.

**UNKNOWN:** the exact deployed revision during the earlier review; current public database/publication records; which filter/source caused each missing result; the approved playable release; the authoritative current client/world pair; final commercial/gameplay claims; the current completeness of every authenticated screen. This project is not an exhaustive runtime/security audit.

The earlier narrow phone capture is not used as proof of a responsive defect: viewport, scale and document width must be measured together before attributing clipping to the site.

## 2. Two delivery outcomes, not one misleading DONE

The project supports an honest **presentation-ready website for an explicitly approved stage**, followed by **playable-release readiness** when the real dependencies are satisfied. A pre-launch presentation does not complete the playable-release obligation or the global Portal programme.

| Dimension | Required result | Must not be substituted with |
| --- | --- | --- |
| Product truth | Approved stage, profile and claims with named evidence | Missing runtime data interpreted as a launch announcement |
| Presentation | Accepted real responsive pages with final copy and approved media | A standalone mockup, screenshot recorder or CI success |
| Content | Required localized content visible through actual publication | Drafts, a route inventory or a populated test fixture |
| Player journey | Evidence for the supported account/client/character/login/world-entry path | Portal account tests relabelled as game E2E |
| Operations | Authorized deployed release, content/media identity and recovery proof | Image build success or a merge alone |

The actual release stage is unresolved input IN-01. Do not silently choose pre-launch merely because client or world data is unavailable.

## 3. Product and interaction design

### 3.1 Information architecture

Recommendation: keep the existing routes and module-owned navigation registry; present approximately five primary groups: **Game / Gra**, **News / Aktualności**, **Knowledge / Wiedza**, **Community / Społeczność**, **Help / Pomoc**, plus account and one stage-appropriate primary action. Exact wording and grouping are approved in the pilot, not hard-coded as a new route contract here.

Existing required capabilities remain reachable even if grouping changes. Do not remove literal route references needed by coverage checks or weaken permission-based visibility. Account/security/admin remain task-oriented rather than promotional. Return visitors get direct account, characters and download shortcuts without adding unapproved personalization.

### 3.2 Homepage composition

The first viewport should explain the game with one concise lead and one valid primary action, with an approved location illustration rather than an oversized wordmark. On mobile, prioritize readable copy and the action; use a separately composed crop, not a desktop panel cut off at the edge.

| Section | Content/data contract | Intended player result |
| --- | --- | --- |
| Hero | Approved stage and identity; real location reference; localized copy | Understand what Oteryn is and find the valid next step |
| Game at a glance | Approved profile, rules/progression descriptors and supported access | Know whether this is the type of server the player wants |
| Why Oteryn | Three or four verified differentiators with evidence | Compare concrete benefits, not interchangeable fantasy slogans |
| See the game | Captioned actual-client screenshots; optional click-to-play clip | Understand real gameplay and distinguish it from key art |
| Start playing / join tests / follow preparation | Exact stage-specific journey and dependencies | Complete a supported next action without a dead end |
| Current information | Published news/notices/events and typed service signals | See useful current information without repeated empty panels |
| Help and community | Verified active destinations and useful answers | Resolve installation/access questions and find other players |

Status is a compact useful signal, not a duplicate large placeholder in several sections. Genuine outages remain visible where they affect player action. No random rates, player counts, start dates, PvP/reset policies or monetization promises are added to fill space.

### 3.3 Shared visual system

Retain the Oteryn mark and navy/gold direction pending the pilot acceptance. Define a restrained heading/body type hierarchy, a shared spacing scale, container widths, distinct primary/secondary/destructive actions, visible focus, readable tables and explicit form states. Avoid using one decorative card component for unrelated information types.

Use three page families: promotional world presentation, editorial/reference reading, and task/dense-data screens. A neutral approved type pairing and subtle borders are starting recommendations, not final visual approval. Large imagery belongs where it explains the world; login/MFA/payment actions prioritize clarity.

### 3.4 Stage, health and action matrix

| Approved stage | Dependency observation | Required behaviour |
| --- | --- | --- |
| Preparation | No playable client/world approved | Present approved materials and development information; no fake Play now flow |
| Testing | Approved client/access/world combination | Show testing label, eligibility and exact test instructions |
| Testing or playable | Status is stale/unavailable | Explain that the state cannot currently be verified; do not manufacture offline/zero or change the release stage |
| Playable | Client withdrawn or required guide missing | Preserve existing valid account/help paths, report the affected step, and reject release acceptance until repaired |
| Playable | Confirmed maintenance | Show the actual maintenance information; no invented reopening time |
| Any | Optional event list legitimately empty | Omit/reduce the empty promotional panel while preserving the events destination and truthful response |

These are presentation rules, not a new login/admission authority. Never authorize access because a marketing readiness flag is true. Choosing persistent stage ownership or a new cross-module readiness contract requires checking accepted architecture first.

## 4. Media and content production package

### 4.1 Media inventory

The following quantities are initial proposed deliverables for the pilot/release, not claims that the files already exist.

| Asset ID | Initial deliverable | Required provenance and approval |
| --- | --- | --- |
| ART-01 | One hero illustration with desktop/tablet/phone crops | Chosen real location; map/profile revision or approved bounded reference; landmark comparison; publication-rights disposition |
| ART-02 | Four to six actual gameplay captures | Approved client/build and world; scene caption; no private names/chat/account data; explicit publication approval |
| ART-03 | Screenshots for each actual installation/login step, usually five to eight | Same supported client and instructions; redact credentials, tokens and private identifiers |
| ART-04 | Three or four differentiator media treatments | Each bound to a verified product claim, not invented features |
| ART-05 | Article/news image rules plus a truthful neutral fallback | Editorial source, crop/focal point, descriptive/decorative classification and locale suitability |
| ART-06 | Social preview image and existing-brand icon treatment | Source rights, readable small-format result and correct metadata |
| ART-07 | Optional short click-to-play gameplay clip | Only after baseline image/site budgets pass; no compulsory autoplay or new tracking dependency |

For a proposed Thais scene, confirm that this is the intended place in the chosen Oteryn profile. Obtain bounded references showing streets, characteristic buildings, coastline/topography and viewpoint. Approve the landmark sheet before detailed art. A full-world map transfer or external repository access is not implied.

For each asset record: stable asset ID, source/source revision, creator or provenance reference, intended use, rights disposition, illustration versus gameplay, intrinsic dimensions, variants/crops, file digest, locale/alt-text treatment and approver evidence. Do not publish unresolved material. Store approved assets using existing media mechanisms where applicable, not a second ad hoc image system.

### 4.2 Editorial inventory

Required copy has an owner, source facts, EN and PL versions, review state, intended publication target, revision and an observed published result. Draft completion and publication completion are separate.

| Content ID | Required content | Publication acceptance |
| --- | --- | --- |
| COPY-01 | Homepage and approved differentiators | Every claim maps to IN-02; both locales read naturally |
| COPY-02 | Server information/rules/profile | Matches approved profile; changes have effective dates where needed |
| COPY-03 | Download/install instructions and requirements | Names the supported release/platform; actual file resolves |
| COPY-04 | Getting started through the actual first-login path | Published typed CMS page and fresh localized version; tested by following it |
| COPY-05 | Practical FAQ and access/install troubleshooting | Valid recovery/help paths, no internal architecture terminology |
| COPY-06 | Account/security/error/empty-state microcopy | Clear next action without leaking sensitive implementation details |
| COPY-07 | Required support/legal/contact content | Appropriate responsible-owner review; no invented operator details or legal compliance claim |
| COPY-08 | Whole-navigation/footer/metadata EN/PL review | No unintended language mixing, keys, mojibake or unconfigured required content |
| COPY-09 | Wiki/reference launch inventory | Exact adopted catalog/profile coverage; measured gaps, not fixture counts |

Proposed Polish microcopy: "Załóż konto", "Pobierz klienta", "Pierwsze logowanie", "Znajdź gildię". Use the corresponding stage-appropriate meanings in English. These examples do not approve a launch stage or actual feature availability.

Source-level UTF-8 validity alone does not detect text already double-encoded as valid UTF-8. Test suspicious patterns and known regressions, inspect rendered EN/PL output, and review meaning manually. Do not blindly replace characters across player-authored content or databases.

Publication must use authorized CMS/distribution actions and retain prior revisions for rollback. Do not seed demonstration news, fake events or guessed product facts into the live environment.

## 5. Page-by-page coverage and acceptance

This is the initial review matrix, not a replacement for the canonical per-capability inventory. Before PEX-06, map every in-scope live route to one family, actor, state, locale and owner. Conditional/deferred features keep their accepted dispositions.

| Family | Required experience | State/data evidence |
| --- | --- | --- |
| Home and game information | Identity, rules, differentiators, useful next step | Approved stage plus valid public data or explicit unavailability |
| News/article/events/notices | Readable index/detail/archive and truthful dates | Published content, real empty result, failed dependency |
| Downloads | Supported-platform choice, version, instructions and real file | Eligible release/artifact, withdrawn/unsupported/unavailable cases |
| Getting started | Complete readable guide with actual-client captures | Missing/unpublished/stale-translation cases preserved and actionable |
| Wiki/search/catalog | Useful search, categories, detail pages and related data | Active adopted snapshot/profile, coverage inventory, zero results versus outage |
| Characters/highscores/online/guilds | Find and compare player-relevant public information | Correct public permissions, pagination, fresh versus unavailable data |
| Registration/login/recovery/verification/MFA | Clear accessible task flow and recovery | Valid, invalid, expired, rate-limited and denied cases in isolated tests |
| Account/characters | Supported actions and understandable progress | Owner-private state, conflict/denied/unavailable/recovery behaviour |
| Support/reporting/legal | Useful guidance and valid submission/read paths | Approved publication, form validation and permissions |
| Commerce/Bazaar | Existing approved scope only, no implied activation | Separate disposition/security/financial gates remain intact |
| Existing player tools | Complete accepted workflows, not new feature promises | Current capability dispositions, input/version/privacy boundaries |
| Administration | Efficient publication, translation, preview and task navigation | MFA, least privilege, auditability, stale-update/revocation cases |
| Global/error/maintenance | Navigation, footer, 404/403/unavailable states | Safe messages, meaningful escape/help action, no sensitive leaks |

A page receiving HTTP 200 is not proof that it is useful. Every required destination needs a result-level assertion. A legitimate empty community/event list need not block launch; a missing required download or guide does.

## 6. Required inputs and decision ownership

Detailed machine-readable requests are in `PORTAL_PLAYER_EXPERIENCE_READINESS_INPUTS.json`. It is an inert planning register with unresolved values, not a new runtime gate, live scheduler or canonical capability-disposition record.

| ID | Missing decision/evidence | Responsible role, not an assigned person | Blocks |
| --- | --- | --- | --- |
| IN-01 | Target release stage, environment and world/profile identity | Product/release owner | Final CTA, publication mode and exact readiness claim |
| IN-02 | Approved fact sheet: real differentiators, rules, progression, supported features and prohibited/unapproved promises | Product/game fact owner | Final promotional copy and feature presentation |
| IN-03 | Chosen location references and approved media/provenance | Art owner plus product/release owner | Authentic hero/gallery publication |
| IN-04 | Approved client/build, supported systems, immutable artifacts and compatibility evidence | Client/distribution owner | Download publication and playable start path |
| IN-05 | Exact account/character/login/world-entry contract and safe test environment | Identity/client/game owners | Full playable journey proof; no personal account reuse |
| IN-06 | Current CMS/localization/catalog publication inventory and authorized publication path | Editorial/catalog owner | Content completeness and deployed content acceptance |
| IN-07 | Exact named launch-capability dispositions and required routes | Product owner via canonical completion scope | Unambiguous scope/acceptance, especially conditional tools and commerce |
| IN-08 | Deployment/read-only inspection permissions, release identity, recovery target and operator responsibility | Environment/release owner | Deployed inspection, publication/activation, rollback and launch |

Resolve retrievable facts from allowed sources first. The owner is not asked to manually reproduce repository information. Product choices and missing external authority need explicit decisions; register them once with evidence. No additional universal scope freeze is created: an unresolved art choice does not block an independently selected localization fix.

## 7. Proposed work packages and dependencies

Package IDs are planning references only. Future implementation requires live selection, current ownership checks and one appropriate task/PR per independently verifiable slice. Role labels below are recommended responsibilities, not launched workers. Effort is relative scope, not an elapsed-time estimate or budget commitment.

| ID / phase | Deliverable | Dependencies / admission evidence | Acceptance | Role / relative scope |
| --- | --- | --- | --- | --- |
| PEX-01 / A | Exact baseline, launch facts, required route/content inventory and targeted world/client/guide diagnostics | IN-01, IN-07, permitted IN-08 reads; record unavailable evidence | Every blocker has a source, owner, next verification and affected claim; no fabricated diagnosis | Product/Platform analyst / M |
| PEX-02 / A | Bounded encoding, mixed-language and internal-copy repair | Current source reproduction and live task selection; independent of final art | Rendered EN/PL regressions pass; semantic/security behaviour preserved | Localization owner / S |
| PEX-03 / B | Approved copy/asset package and landmark-based art brief | PEX-01, IN-02, IN-03; IN-05 for final onboarding captures | Fact-to-copy and reference-to-image review; unresolved assets remain unpublished | Editorial/art owners / M |
| PEX-04 / B | Real homepage, download, guide and representative account pilot | PEX-03; IN-04/05 for final client facts; existing protected preview evaluated | Desktop/mobile and EN/PL real application views accepted by owner before broad propagation | Single UI owner / M-L |
| PEX-05 / C | Repair confirmed world/download/guide publication or integration gaps | PEX-01, IN-04/05/06/08 and existing module ownership | Real observed outcomes; no bypassing eligibility filters or manufacturing world data | Module/distribution/editorial owners / M-L |
| PEX-06 / C | Roll accepted visual system across all in-scope page families | PEX-04 accepted; PEX-02; PEX-05 outputs where required | Full route/state/locale mapping; coherent task, reading and promotional families | Single UI owner / L |
| PEX-07 / C | Complete and publish the approved content/media inventory | PEX-03, PEX-05, IN-06/07/08 | Required public content revisions and adopted catalog coverage verified | Editorial/catalog owner / M-L |
| PEX-08 / D | Actual player journey and recovery proof | PEX-05/06/07, IN-01/04/05/08; all relevant external authority | Correct client/account/world pairing reaches the intended result; evidence redacted | Independent journey reviewer / M |
| PEX-09 / E | Release-candidate visual/content/accessibility/performance review | PEX-06/07/08 for playable release; scoped pre-launch exception recorded as pre-launch only | Material findings resolved; product and technical evidence bound to candidate | Independent reviewer and product owner / M |
| PEX-10 / E | Protected integration, authorized deployed acceptance, recovery and closeout | PEX-09, IN-08 and normal repository/environment gates | Deployed release plus content/media revisions match; ordinary/cached refresh and required journey pass | Release owner / M |

Recommended dependency order for this experience scope: PEX-01 unlocks PEX-03 and PEX-05; PEX-02 may proceed independently after canonical selection. PEX-03 -> PEX-04 -> PEX-06 is the design path. PEX-05 plus PEX-03 -> PEX-07 is the publication path. These converge at PEX-08 -> PEX-09 -> PEX-10. Upstream canonical security/authority/dependency priorities remain unchanged.

Do not dispatch all packages together. Start with one coordinator/analyst and one selected writer. Parallel art/editorial or module work is useful only with disjoint paths and explicit handoffs. Only one writer owns shared layouts/CSS/navigation at a time. Review must be independent of the implementation being accepted. No background worker or external paid service is started by this plan.

At the end of PEX-01, estimate each selected slice from actual route counts, media availability, publication state and environment access. Separate implementation effort, artwork/content production, CI/queue latency and owner/external waiting. No honest calendar completion date can be committed from the currently unresolved inputs.

## 8. Implementation and migration strategy

Retain Laravel/Blade and current security/domain owners. Proposed edit surfaces, to verify when a task is selected: `resources/views/**`, `resources/navigation/**`, `lang/**`, owning `public/css/**` and approved `public/images/**`. Module fixes are restricted to the exact reproduced gap; no unrelated account/payment/game rewrites.

Use the existing code-owned homepage preview/selection mechanism where suitable. Do not expose public arbitrary-template selection or widen administrator permissions. A new proposed presentation can be reviewed on an isolated application instance; runtime databases and personal browser sessions are not design fixtures.

Consolidate CSS component by component. Keep content-hash asset versioning and introduce appropriate responsive source/crop choices. Current design-specific tests such as a fixed 626 px art limit must be deliberately updated to the approved image contract, retaining image decoding, density-appropriate quality, layout stability, no overflow, cache and keyboard checks. Do not delete or skip a failing test merely to get green CI.

Content rollout is a separate controlled change set: snapshot approved public revisions, prepare unpublished EN/PL drafts and matching assets, verify links and source-version freshness, then publish only through the authorized existing workflow. Use additive/backward-compatible schema changes only if the selected gap proves they are necessary. A routine content correction should not require a new generic readiness service.

Before merging each slice, inspect the full diff, preserve routes and negative security assertions, run the focused checks plus required exact-candidate gates, and use the existing protected integration path. Current main movement alone is not a reason to replace the canonical branch or repeat unchanged expensive work.

## 9. Acceptance specification

### 9.1 Visual and product approval

Owner approval is tied to a known rendered candidate with final copy/media, not to an isolated generated image. Review homepage, a long article/guide, download page, dense listing/detail, account/MFA and admin task surface. Verify landmarks against the accepted reference and compare illustration versus actual gameplay explicitly.

Pilot rejection leads to a bounded iteration in the pilot, not propagation across every route followed by another global redesign. A small qualitative review with new observers asks them to explain Oteryn, find its rules, identify a real differentiator and find the supported next action. This is usability evidence, not a popularity forecast.

### 9.2 Automated and manual test matrix

| Test ID | What must be proven | Evidence boundary |
| --- | --- | --- |
| UX-01 | Correct stage-specific CTA under good, missing, stale and unavailable inputs | Unit/feature plus browser tests; never marketing as admission authority |
| UX-02 | Required published routes contain the expected useful content in EN/PL | Real publication-state test; seed-only screenshots are not deployed proof |
| UX-03 | Correct artifact selection, real download and revoked/unsupported cases | Separate browser distribution from updater signing/activation evidence |
| UX-04 | Correct Unicode and no accidental internal-language/locale leaks | Source scan plus rendered review; review false positives |
| UX-05 | Navigation, keyboard, focus, no-JS fallback, reduced motion and text zoom | Retain existing assertions and inspect interactions manually |
| UX-06 | Reflow and image behaviour at actual known viewport/DPR | Capture `innerWidth`, client/scroll width, viewport height, zoom/DPR and screenshot dimensions together |
| UX-07 | Auth/account privacy and negative paths remain intact | Isolated authorized accounts; no personal data or tokens in artifacts |
| UX-08 | Correct client/login/world entry and recovery | Exact approved build/profile on safe real environment; no Portal-only substitution |
| UX-09 | Asset versions, missing-media fallback, current/previous release cache behaviour | Fresh and warmed cache; record actual requested/served versions |
| UX-10 | Performance, deployed identity and rollback | Lab versus field labels; exact code/content/media identities; authorized recovery proof |

Recommended layout sample: 320, 390, 820, 1440 and 1920 CSS pixels; known heights; DPR 1 and representative DPR 2; text enlargement and reduced motion. Use the existing Chromium/Firefox/WebKit coverage where applicable. Do not multiply every state by every browser/viewport without a risk-based matrix; do not drop required exact-head checks.

Accessibility target: WCAG 2.2 AA, including keyboard, unobscured focus, labels/errors, contrast and reflow at 320 CSS pixels, with the standard's exceptions for genuinely two-dimensional content. Automated scans are supporting evidence, not certification. See [W3C WCAG 2.2](https://www.w3.org/TR/WCAG22/) and [Reflow guidance](https://www.w3.org/WAI/WCAG22/Understanding/reflow).

Performance target: LCP <= 2.5 s, INP <= 200 ms, CLS <= 0.1 at the 75th percentile of sufficient field observations, separately for desktop/mobile. Before field data exists, report repeated fixed-condition lab measurements and missing field evidence; do not substitute Lighthouse TBT for measured INP. See [Google Web Vitals](https://web.dev/articles/vitals). Numeric media/transfer budgets are set after the actual pilot baseline, not invented here. Use responsive images and on-demand optional video meanwhile.

### 9.3 Evidence bundle and acceptance status

For every accepted slice store: task/PR, exact code SHA, environment and observation time, release image/build identity where relevant, content revision IDs, media digests, locale/actor/test matrix, expected outcome, observed result, redacted evidence location, unresolved findings and reviewer/owner acceptance evidence. Do not upload full personal browser profiles, credentials or private account HTML.

Required result categories in the release report: `technical_validation`, `visual_approval`, `published_content`, `player_journey`, `deployed_environment`, and `production_authority`. Report each separately with evidence and PASS / FAIL / NOT_RUN / BLOCKED / justified NOT_APPLICABLE. These are reporting categories, not replacement GitHub required statuses. No generic DONE can stand in for missing categories.

For a preparation-stage release, playable world-entry proof is outside that particular presentation acceptance, but remains explicitly NOT_PROVEN for playable/global completion. Do not silently defer an already-required capability.

## 10. Rollout, rollback and stop conditions

### R0: isolated candidate

Build the pilot against controlled fixture data and approved media. Prepare content drafts and a manifest without touching the live publication state. Capture the visual baseline and verify the existing preview permissions.

### R1: authorized staging rehearsal

Only with exact environment authority, record the current proven code/image, public content revisions, media digests and recovery target. Deploy the candidate through the normal guarded route, publish the approved content set, and verify eligibility/readiness separately from HTTP health. Staging success is not production approval.

### R2: candidate freeze and independent acceptance

Freeze the code/content/media combination being reviewed. Avoid unrelated changes and repeated no-op commits. Resolve material findings and refresh only affected evidence. Required merge-group/exact-head checks and the existing integration authority remain mandatory.

### R3: authorized release and readback

Before production/public activation, obtain the separate release decision. Confirm exact target and last-good identities and required backup/restore/recovery evidence. Use existing rollout capabilities; do not assume traffic splitting or percentage canaries exist. After release, inspect public and permitted authenticated routes, actual asset URLs, locale/content revisions, supported download and the applicable player-start path. Record normal/cached refresh results on desktop/mobile.

### R4: rollback or closeout

Stop acceptance for wrong client/profile, broken required onboarding, private data exposure, auth regression, missing required media/content, unreadable reflow or an unresolved material quality finding. Restore the previously proven compatible code/image through the authorized release process when rollback is required. Restore prior approved public content/media references where necessary; code rollback alone does not roll back CMS publication. Do not automatically restore a full database or erase intervening user data. Rollback must retain artifact eligibility/signing/security boundaries.

The homepage template rollback is only one presentation capability; it does not prove whole-site, content or schema recovery. Each selected slice records its actual rollback mechanism. Release acceptance ends only after healthy readback, disposition of findings and intentional task/PR/source-branch closeout. Monitoring ownership belongs to the release operator; this document starts no unattended monitoring job.

## 11. What can proceed next

The next recommendation is **PEX-01 baseline/input reconciliation**, with **PEX-02 localization repair** as an independently eligible candidate only after normal live selection and implementation authority. In parallel, approved art references and product facts can be prepared without changing the site. Final art production, published gameplay promises and a playable-release claim wait only for their specific dependencies, not for an invented universal freeze.

The first material owner decision to resolve is IN-01: preparation, testing or playable launch, naming the intended environment/profile. Retrieve existing evidence first; ask one bounded decision question when no accepted record resolves it. The eight input requests make the remaining needs explicit without forcing the owner to write technical documentation.

**Planning result:** the product direction is now decomposed into concrete inputs, page/media/content deliverables, ten dependent work packages, acceptance evidence and controlled rollout/recovery. **Not claimed:** completed artwork, completed copy publication, selected runtime workers, a repaired portal, successful game login, a deployment or global Portal completion.
