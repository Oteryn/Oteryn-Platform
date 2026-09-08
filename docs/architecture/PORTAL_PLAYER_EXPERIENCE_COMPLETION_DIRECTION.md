# Oteryn Portal Player Experience Completion Direction

## Status and authority

**OWNER-APPROVED PLAYER-FACING COMPLETION DIRECTION — subordinate to accepted architecture, `docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md`, the live `OTERYN_PORTAL_COMPLETION` selector, repository governance, and task-specific authority.**

Governing documentation Issue: #1356.

This document records the player-facing product direction that must guide future Portal Completion work. It is not a second selector, scheduler, runtime status source, deployment authority, game-server authority, or permission to invent gameplay claims. It does not reopen or resurrect the terminal `PORTAL-POLISH` task.

Current/runtime facts still come from the appropriate authoritative sources and live GitHub/environment evidence. When this document conflicts with a newer accepted owner decision or higher architecture authority, the newer/higher authority wins and this document must be reconciled.

## Product outcome

The Portal is not complete merely because every route exists, shared CSS is polished, browser automation passes, or a deployment succeeds. It is complete for a player only when the whole public experience coherently answers:

1. **What is Oteryn?**
2. **Why should I play here?**
3. **How do I start?**

The intended result is a modern Oteryn game website with a recognizable connection to the actual game world and real player journey, not a generic fantasy template or a technical service portal decorated with fantasy artwork.

The existing Laravel/Blade application, domain boundaries, account/security behavior, CMS, authorization, public data contracts and other accepted Platform architecture remain the foundation. Player-experience completion should improve and complete those surfaces rather than rewrite the Portal framework solely for presentation.

## Evidence that motivated this direction

A live public-Portal review on 2026-09-08 found that the deployed presentation could look coherent while still failing the intended player-product test. Observed examples included:

- a strong fantasy hero composition followed immediately by an empty/no-world state;
- a download center with no current downloadable client;
- a getting-started route whose player-visible result was an unconfigured editorial page rather than onboarding;
- visible Polish mojibake/broken-character strings and mixed-language/internal-technical wording;
- repeated generic fantasy imagery that communicated atmosphere more strongly than the identity of the actual game and world;
- multiple empty modules occupying prominent homepage space even when they provided no current player value.

These observations are time-bound evidence, not permanent claims about the current environment. The response must fix the product model and acceptance criteria so future work cannot be declared complete merely because the same screens render without technical errors.

## 1. Separate release stage from service/world health

The Portal must represent two different concepts independently:

### Release stage/readiness

Examples include preparation, closed testing, open testing, playable launch, or another explicitly approved release stage. This is a product/release decision and must come from accepted owner/release authority.

### Current service/world health

Examples include available, maintenance, stale, dependency unavailable, or another typed runtime state. This comes from the relevant operational/runtime evidence source.

The Portal must never infer release stage from missing runtime evidence. Likewise, runtime unavailability must not be converted into `offline`, `0 players`, `no activity`, success, or another fabricated fact.

Player-facing composition may change with a proven release stage:

- **Before playable launch:** present the game honestly, show approved development/release information and available materials, and do not pretend that a playable live world exists.
- **Testing:** clearly label testing, eligibility/access rules and the correct test-client/onboarding path.
- **Playable launch:** a prominent play/download path is valid only when the corresponding client/account/onboarding/world-entry path is actually supported and proven for that release.

An attractive pre-launch site is valid. Pretending a blocked or unknown play path is complete is not.

## 2. Visual identity: modern Oteryn with recognizable game-world truth

Retain the useful brand foundation — including the Oteryn mark and the dark navy/gold direction unless a later owner-approved art decision changes it — while moving away from a generic fantasy-site result.

### World illustration

When artwork claims or strongly implies a specific real game location, the production process should be:

1. collect an approved reference from the intended Oteryn/Tibia-world location;
2. identify the recognizable layout, landmarks, streets/buildings/topography and composition anchors;
3. create or select artwork that preserves those recognizable anchors;
4. apply the intended cinematic/illustrative lighting and finish;
5. compare the final art against the approved location reference before publishing it as that location.

A prompt such as “make a fantasy Tibia-like city” is not sufficient evidence that an image represents a real place in the game.

### Gameplay material

Illustrative/key art and actual gameplay evidence serve different purposes and must not be conflated.

- **Illustrative world art** may establish mood and brand identity.
- **Gameplay screenshots/video** must show the real approved client/game presentation and remain clearly distinguishable from illustration.

The public experience should contain real gameplay material when available and authorized so a prospective player can see what actually happens after launching the client.

### Asset set

The visual system should be designed as a coherent asset family rather than one replacement hero image. Expected material may include:

- desktop and mobile hero crops/compositions;
- real gameplay screenshots or short approved clips;
- article/news/event imagery with clear editorial provenance;
- guide/onboarding screenshots from the supported client;
- item/creature/vocation imagery only when the applicable game-data/profile provenance is known;
- supporting illustrations that remain consistent in palette, rendering style and subject truth.

Do not add heavy autoplay video, particles or gratuitous animation merely to appear modern. Performance, clarity and product truth take priority over spectacle.

## 3. Homepage must support the player decision

The homepage should not function as an equal-weight catalogue of every implemented module. It should guide a new or returning player through the most important questions and actions.

### Recommended composition

#### A. Hero / first viewport

Show:

- the Oteryn identity without allowing the wordmark to consume most of the viewport;
- a recognizable approved world/game visual;
- one concise description of what Oteryn is;
- one primary call to action appropriate to the proven release stage;
- a secondary action only when it helps the player continue the current journey.

Avoid a situation where the hero promises a “living world” while the next prominent component says there is no configured world. If status is genuinely unknown/unavailable, compose that truth deliberately rather than as an accidental contradiction.

#### B. Oteryn at a glance

Present only confirmed player-relevant facts such as the approved game/world profile, ruleset, progression character, availability or other explicitly accepted descriptors.

Do not invent marketing facts such as “no resets”, “no pay-to-win”, “perfect balance”, rates, launch dates or other claims unless the appropriate authority has approved them and the product actually satisfies them.

#### C. Why Oteryn

Show three or four real differentiators. Each should be backed by an example, screenshot, approved feature or other truthful evidence. Generic claims such as “unique adventure” are insufficient on their own.

#### D. See the game

Use a small, high-quality gameplay gallery or short media presentation with captions explaining what the player is seeing. The purpose is product understanding, not decorative volume.

#### E. How to start

Provide the real supported sequence from account to client/onboarding. Each visible step must lead to a useful result appropriate to the release stage.

#### F. What is happening now

Show real news, notices, maintenance or events when useful. Before launch, an approved development/release journal may be more useful than empty event/status modules.

#### G. Community and help

Present only active/approved community/help routes and the most useful new-player answers.

### Returning player shortcuts

Returning players should be able to reach account, characters, downloads and current information quickly without forcing the homepage to become a dense personalized dashboard. More advanced personalization remains subject to the existing Portal Completion scope/architecture gates.

## 4. Complete each major page family according to its job

A shared visual system should not make every route look like the same hero/card template. Use coherent families with different information density and purpose.

### Game/world presentation

Explain the approved world/game profile in player language. Show real rules and differentiators only when confirmed. Runtime/server metadata should be presented where useful, not as internal infrastructure diagnostics.

### News, articles and events

Use readable long-form typography, meaningful media, dates, hierarchy and archives. Events shown as current/upcoming must be actually published/scheduled through their authoritative content path.

### Downloads

A playable release requires a clear, current, approved client path for each supported platform. Show version/release information, requirements, installation help and appropriate integrity metadata without making checksum/provenance language the primary marketing copy.

Unsupported or unavailable platforms must not look available merely because a tab exists.

### Getting started

Provide a complete guide from account creation through the supported first login/world-entry path. Use screenshots from the supported client when available. Include common recovery/troubleshooting guidance where appropriate.

A technical placeholder such as “managed page not configured” is not an acceptable final result for a required onboarding route.

### Wiki and game catalog

Provide useful search, categories and truthful content coverage. Item/creature/NPC/other facts must remain bound to the accepted authoritative catalog/profile/version evidence. A few populated examples must not be represented as complete reference coverage.

### Characters, guilds, rankings and community reads

Optimize for lookup and comparison. Distinguish no results from unavailable/stale dependencies. Player-facing text should describe the feature, not the internal read-only boundary implementation.

### Account and security

Use a calmer task-oriented presentation. Prioritize characters, next actions, account status, recovery and security. Decorative art must not obstruct high-consequence actions.

### Support, legal and error states

Provide actionable guidance and real contact/reporting paths. Error/unavailable content should tell the player what happened at the appropriate abstraction level and what they can do next. Internal implementation terminology is not player copy.

### Commerce

Expose commerce only within an accepted product disposition and the existing security/legal/operational gates. A designed page is not authority to activate real payments or advertise unapproved commercial behavior.

### Administration

Keep the same design language but prioritize operational efficiency. Administration should make player-visible publication gaps obvious where feasible: missing locale, unpublished required guide, unavailable client release, missing required image, or other launch-readiness omissions. This document does not itself require a new generic “readiness module”; implementation must remain bounded to accepted architecture and selected slices.

## 5. Player-facing copy is a product surface

EN/PL content quality is part of completion, not a cosmetic follow-up.

### Required editorial rules

- remove mojibake/broken encoding from published strings;
- remove accidental language mixing where a translation is expected;
- write for a player rather than an internal system operator;
- preserve technical integrity/security details where needed, but move them to the appropriate secondary detail surface;
- do not expose implementation vocabulary such as “Platform identity”, “approved process”, “read-only game-data boundary” or CMS-internal state as primary marketing/onboarding copy;
- do not make unverified gameplay, economy, balance, launch or commercial claims.

Examples of the preferred abstraction level:

- “Create an account and begin your Oteryn journey.” rather than “Create a secure Platform identity.”
- “Download the client for your system.” rather than leading with artifact/checksum implementation terminology.
- “Find a guild and meet its members.” rather than explaining the internal read-only data boundary.
- “We cannot read the world status right now. Try again later.” when the status dependency is unavailable, rather than inventing an offline/empty state.

Content must be proven through the actual publication path. A correct repository translation file is not enough if the deployed player route is still empty or unconfigured in CMS state.

## 6. The real player journey is a release acceptance path

The applicable journey is:

**Portal -> account -> approved client -> supported character flow -> login -> world entry**

The exact steps must follow the real account/client/game contract rather than an assumed generic OTS flow.

### World/status gap

When the public Portal has no usable world result, determine whether the cause is configuration, publication/visibility, environment routing, stale/unavailable runtime input or another verified condition. Those are investigation categories, not preselected diagnoses.

### Client gap

Before declaring a playable path complete:

- identify the approved client release;
- prove compatibility with the intended release/world path;
- publish it through the accepted distribution process;
- verify the public download result;
- verify installation/launch on the applicable clean test environment where authorized.

Do not substitute an arbitrary archive merely to make the page non-empty.

### Getting-started gap

Write the guide from a real successful setup/login path and publish it through the actual editorial mechanism. Then test the journey by following the published guide.

### Cross-repository/external dependencies

If completion requires evidence or mutation owned by a game/client/server repository or another protected environment, keep the exact dependency explicit. Platform-only work may continue where independent, but a “ready to play” claim remains false/unknown until the required authorized evidence exists.

## 7. Empty-state policy

Not all empty modules have equal product impact.

### Optional empty content

Examples may include no upcoming event or no current announcement. When absence is legitimate and unimportant to the current decision, the homepage may reduce or suppress that empty module compositionally rather than devote large space to it.

### Required journey gap

Examples include a missing required client release or missing required onboarding content for a release that claims players can start playing. These are blockers to that claim and must not be hidden as a visual-design workaround.

### Runtime unavailable/stale state

Preserve the typed unavailable/stale semantics. Do not present dependency failure as normal empty content.

## 8. Implementation boundary

Retain the accepted Laravel modular-monolith architecture and existing ownership/security contracts. Player-experience work should reuse existing routes, CMS, localization, account/auth/RBAC, catalog and public-data abstractions unless a selected slice proves a separate architecture decision is required.

Avoid accumulating a new endless presentation override layer. When changing the design system, progressively move final rules into the appropriate owning components/stylesheets while preserving regression coverage and avoiding a broad unrelated rewrite.

UI work must not silently change authorization, payment, account-security, game-data or game-server semantics.

## 9. Acceptance model

Automated tests are necessary but are not the final visual/product verdict.

### A. Visual and identity acceptance

Review real rendered pages with target copy and media at the relevant desktop/mobile sizes. The owner must be able to approve that the result looks like Oteryn and appropriately represents the intended game/world. A passing screenshot recorder alone is not this approval.

### B. Content and usability acceptance

Required navigation destinations must produce useful player results. No broken-character strings, unintended mixed-language copy or internal technical placeholders may remain in the accepted release surface.

For major onboarding/navigation changes, use a bounded qualitative review: a person unfamiliar with the implementation should be able to explain what Oteryn is, identify why it may interest them and find the next valid step without internal guidance. This is a usability check, not a prediction of commercial popularity.

### C. Functional, responsive and accessibility acceptance

Retain existing automated browser/security/negative-path validation and verify the real page behavior at known viewport dimensions. A cropped screenshot is not equivalent to a responsive test.

Use the repository's accepted accessibility target; where no stricter local requirement applies, aim for WCAG 2.2 AA behavior including keyboard access, visible focus, contrast and robust reflow at narrow widths. Automated scanners are supporting evidence rather than a complete accessibility certification.

### D. Performance acceptance

Use appropriately sized responsive images and avoid heavy media blocking primary content. Measure performance rather than claiming improvement from CSS byte reductions or visual inspection alone.

For public real-user performance, the product target should align with current Core Web Vitals guidance where applicable: LCP <= 2.5 s, INP <= 200 ms and CLS <= 0.1 at the 75th percentile, evaluated separately for mobile and desktop when sufficient field data exists. Laboratory measurements may be reported before sufficient field data, but must not be relabeled as field results.

### E. Deployed acceptance

After protected integration and an authorized deployment, verify the exact deployed release on the real target URL/environment, including ordinary refresh/cache behavior. Repository completion, staging acceptance and production/public launch remain distinct evidence states unless the applicable authority explicitly combines them.

## 10. Delivery sequence

Future implementation should decompose this direction into bounded existing-programme slices rather than one giant redesign PR.

### A. Facts and blockers

Deliver:

- confirmed release stage and approved public claims;
- confirmed required player-start path for that stage;
- diagnosis/ownership of observed world/client/getting-started gaps;
- immediate correction of obvious player-facing localization/encoding defects where independently safe and selected;
- explicit list of external/client/game dependencies.

Exit when the Portal team knows what it may truthfully promise and what dependencies block the next player outcome.

### B. Direction on real screens

Deliver the proposed target system first on:

- homepage desktop/mobile;
- downloads;
- getting started;
- one representative account/task surface.

Use real/approved copy and media. Obtain product/visual acceptance before propagating the direction across every page family.

### C. Complete the agreed surface

Apply the accepted system to the required page families, content, EN/PL copy, shared components and truthful empty/unavailable states. Keep deferred/conditional features within their existing Portal Completion dispositions rather than expanding scope because a design exists for them.

### D. Player journey

Prove the applicable actor-to-result path on an authorized environment. For a playable release, this requires the real supported path through client/login/world entry rather than only a Portal mock or isolated account E2E.

### E. Release acceptance

Complete protected integration, authorized deployment, deployed review and separate visual/content/function/environment acceptance. Do not close the programme based on CI alone.

## 11. Relationship to existing Portal Completion scope

This direction changes **how player-facing completeness is evaluated**; it does not reorder or self-activate existing Portal Completion workstreams.

In particular:

- `OTERYN_PORTAL_COMPLETION` remains the sole live selector;
- `PORTAL_COMPLETION_DELIVERY_PLAN.md` remains the capability/dependency order;
- required/conditional/deferred/rejected dispositions remain owned by the existing completion scope and accepted architecture;
- forum, World Hub, optional PlayerCompanion follow-ups, commerce activation and other conditional/deferred work do not become required merely because this document discusses player experience;
- production/public-edge proof still requires its separate authority;
- external/client/server evidence still requires separate exact authorization when outside Platform scope.

## 12. Known inputs required before final player-facing release copy can be completed

The following must be resolved from accepted authority/evidence rather than guessed:

- the release stage being presented;
- approved game/world profile and public rules;
- approved concrete Oteryn differentiators;
- approved world/location reference material and asset provenance;
- the supported client release/distribution path;
- the supported account/character/login/world-entry sequence;
- launch-scope decisions for optional/conditional capability families;
- any claims about reset policy, rates, balance, economy, monetization, events, dates or other gameplay/commercial promises.

These unknowns do not prevent safe independent UX/editorial/platform work from progressing, but they prevent unverified claims from being published as product truth.

## Completion principle

A future agent must not report the Portal as fully player-ready solely because a visual task, route matrix, screenshot suite, CI run or deployment is terminal. The relevant release is player-ready only when the selected Portal Completion scope is terminal **and** the applicable real player experience — truthful presentation, required content, supported start path, visual/product acceptance and deployed evidence — has been proven for that exact release/environment.
