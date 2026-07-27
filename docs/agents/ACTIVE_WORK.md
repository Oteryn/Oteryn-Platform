# Oteryn Platform Active Work

Convenience index only. Individual active task records, live PRs and Git state are authoritative.

## Active tasks

- `OTERYN-20260727-synology-production-target-preflight` — live local Synology host/runtime, storage, restore-drill and rollback-readiness validation tracked by Issue #238; staging-only with no public exposure.

## Closed acceptance and release-preparation follow-ups

- PR #67 / `517968539bdfd7d189677b669bf0899c35fccec1` — issues #68-#70 closed with exact-SHA production-like browser acceptance evidence classified `STAGING_PROVEN`.
- PR #73 / `06d8d94aafd73de996eb4ea93705e8a45fbadafb` — issue #71 closed with controlled Platform DB outage evidence classified `STAGING_PROVEN` for that staging failure path only.
- PR #74 / `24eaa4ca5e38bb255db95a989c0ff02e954360f3` — issue #72 closed with focused CMS publication-state and privileged-audit regressions; no runtime defect found.
- PR #75 / `4fc6fcccea00bdd8d7679595b92d189cb572dd35` — final Functional Acceptance matrix reconciliation merged; FAV-01 through FAV-05 are closed for the delivered staging-verifiable scope.
- PR #77 / `1e6e21f0963406d4e58c39b347a49cfa4529bd1c` — delivered-surface UI/UX remediation merged with clean browser Visual/Accessibility evidence.
- PR #86 / `5d3628f8c6ba2e454246f24947ebe08ca93cf684` — issue #81 closed; authenticated Account Overview and provisioning-status UX delivered with full production-like browser evidence.
- PR #92 / `c18432df6b387932aa04e1eb269677c9078d9063` — fail-closed non-secret Production Go-Live verification evidence packet prepared; actual production execution remains issue #91.
- PR #94 / `26ff602696c597aac0833415b0a47af5d427a52d` — risk-based E2E architecture plus required bounded Chromium/Firefox/WebKit portability, desktop/tablet/mobile responsive and representative browser-security coverage merged.
- PR #99 / `21d67c7e7edb533f9765ff96417f2ab2fbb1aea8` — issue #98 closed; existing Phase 7 release validation now includes isolated synthetic existing-data upgrade, candidate smoke, old-code rollback smoke against the post-upgrade database and candidate redeploy smoke with durable `STAGING_PROVEN` evidence.
- PR #100 / `8a4fd46db04c8c75a4206be8c0920a96cc473452b` — archived the completed migration/rollback validation task and closed its governance lifecycle.
- PR #102 / `ee235cbbdd379a5047fede98ff79a0e35e22ce76` — issue #101 closed; Phase 7 now proves exact response `X-Request-ID` to matching structured request-completion log correlation with bounded method/status agreement and non-secret `STAGING_PROVEN` evidence.
- PR #104 / `18bd5b2c3b4496677cc58df41fd50c6387e9e6f8` — archived the completed observability-correlation task and closed its governance lifecycle.
- PR #106 / `8030f98d7280c16705f34f2d29c8ebd7fc85f285` — issue #105 closed; required acceptance now includes zero-retry Chromium public dependency resilience proving Canary DB and Redis failure, deterministic restoration and successful browser recovery.
- PR #111 / `740d9879b341d98e4cf0ef0e7f076b43cd86cdaf` — issue #110 closed; required acceptance now includes bounded zero-retry Chromium keyboard/focus accessibility, while scheduled/manual three-iteration zero-retry critical stability and read-only public soak profiles provide future non-blocking evidence.
- PR #142 / `f4570b99c9ef6f222ae3aa9ee1d9a41919768df3` — first-party Wiki architecture, phased implementation plan and standalone implementation-agent prompt merged.
- PR #143 / `434ef4c950b52c1cb77422fda1a17c1d20bba59f` — complete non-commercial public website expansion architecture, delivery programme and first-slice implementation prompt merged.
- PR #194 / `9ed3861cc29dcaf6305c379de2bee5ee5ac923d6` — production-capable published-only English/Polish public Wiki reads, restricted Markdown rendering and locale-isolated search merged.
- PR #195 / `c53e0f2a1a93de9275439aff573e5a713f5621b1` — public Wiki task lifecycle archived.
- PR #196 / `f512f1e3a9bd567d40ddb09b699291c99a1b65f8` — trusted-editor Wiki administration, exact RBAC/MFA lifecycle controls, signed previews, revisions and responsive browser acceptance merged.
- PR #197 / `a9adbe07317cac0311e9dd5761d45ceb8c7203f5` — Wiki administration task lifecycle archived.
- PR #198 / `57716094cde335a0e8a661953bd3a5809ec12cb6` — Issue #145 programme checkpoint refreshed after the public and administrator Wiki deliveries.
- PR #199 / `f66c9944fd8110014773bd7cb7b58c9f49e45af0` — approved private EditorialMedia objects integrated into Wiki editing, transactional references, published-only verified public bytes and signed administrator previews.
- PR #206 / `1d063604a66dd3154f97a6f167377d54131cc516` — homepage announcement/event composition, Download and guild navigation, escaped localized metadata, published-only sitemap, authoritative robots policy and required browser closure merged.
- PR #208 / `f8002191f0e5270dc4191227fd01d5e709ee5ab6` — thirteen source-backed bilingual Wiki launch topics, exact-permission/MFA operator provisioning, conflict-safe idempotency and required cross-browser acceptance merged.
- PR #209 / `a262996eda36fc9430fe1883ea637ffd2f6ff698` — source-backed Wiki content task lifecycle archived and Issue #145 reconciled to staging-only closure.
- PR #230 / `d7984a2def655a01b513cdbc823117f37b90d5d4` — explicit Wiki role bundles, genuine-MFA guarded first-administrator bootstrap and reviewed launch-content installation merged.
- PR #232 / `415aa3febd04c8d9c61082d4a7451352bf084013` — exact-SHA Synology deployment and named-volume live Chromium staging acceptance completed with sanitized PASS evidence.
- PR #233 / `e3e94dae03e0468d71f911ad41e597bb5d802eb3` — temporary final-staging workflows and trigger removed after all required cleanup checks passed.
- PR #234 / `4131a34b8c5f1092a2d0b8fb1bb56785f217b194` — closed Issue #145 state and the single remaining archival action persisted in the final-staging checkpoint.
- PR #235 / `cab40863bd5058209cdcbee1342a54acc814ec01` — final public-staging task record archived and `ACTIVE_WORK` cleared.
- PR #236 / `ee8293d8bbf33c9bc89ca105a0273728bb222f4d` — deterministic reserved-domain DNS, TLS, Cloudflare/WAF/Access and authenticated-origin emulation merged with exact-SHA `STAGING_PROVEN` evidence.
- PR #237 / `f5aeb2e80d4692b3ee6309cc3454aa20697721f2` — edge-security emulation task archived and `ACTIVE_WORK` cleared.

## Current project phase

**Phase 6 — CMS, Admin, RBAC and Audit: COMPLETE**

**Phase 7 — Production hardening and operations: COMPLETE**

The E2E coverage-hardening programme is continuous verification and does not reopen either completed phase.

## Operational release state

- **Production Readiness: STAGING_PROVEN**
- **Functional Acceptance: STAGING_PROVEN for the delivered staging-verifiable functional surface**
- **Visual / UX Acceptance: PASS for the delivered staging-verifiable launch scope**
- **Public Website Expansion Programme: STAGING_PROVEN**
- **Emulated Edge Security: STAGING_PROVEN**
- **Synology Production Target Preflight: IN PROGRESS**
- **Production Go-Live Gate: PENDING PRODUCTION VERIFICATION**
- **Production Verification: REQUIRED BEFORE GO-LIVE**

No staging, local-target or emulation evidence may be promoted to `PRODUCTION_PROVEN`.

## Next work

Complete Issue #238 by executing the sanitized live Synology production-target preflight, persisting exact-run `STAGING_PROVEN` evidence, removing its temporary dispatcher and archiving the task. Issue #91 remains the sole real production execution tracker.

## Remaining cross-repository dependency

The authoritative Platform game-login bridge remains separate and requires explicit authorization before external repository writes if it is part of launch scope.

## Coordination rule

Before starting substantial work, search `docs/agents/tasks/active/**` and open PRs for overlapping paths or intent. Do not claim paths already owned by another active task without explicit coordination.