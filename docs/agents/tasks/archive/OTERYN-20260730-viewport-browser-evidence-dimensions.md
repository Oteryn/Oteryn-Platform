---
task_id: OTERYN-20260730-viewport-browser-evidence-dimensions
status: archived
required_reads:
  - AGENTS.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
---

# OTERYN-20260730-viewport-browser-evidence-dimensions

## Result

Issue #347 was completed by PR #349, squash-merged as `30b9c4767b137cde3035a5529410c7d9add2d5ba` from exact final head `f1524e767107afbf542d0d82e29a0f89eada1fc5`.

The delivered gate prevents a portal surface from claiming a viewport or browser boundary without exact executable evidence or an explicit risk-based portability exclusion.

## Delivered

- one canonical dimension contract for all 27 delivered portal surfaces;
- four sorted surface fragments and 13 executable Playwright profile groups;
- exact file, marker, project, browser, viewport, blocking invocation and zero-retry linkage;
- blocking Chromium desktop `1440x1000`, tablet `820x1180` and mobile `390x844` evidence for all 23 critical rendered surfaces;
- executable Firefox/WebKit coverage or a bounded exclusion rationale;
- six general negative fixtures and two critical-viewport fixtures;
- strict Portal Acceptance integration;
- truthful responsive localization assertions using visible user-operable links;
- removal of the unproven generic Marketplace portability claim.

## First failing invariant

Portal Acceptance run `30532300578` on head `5180093e7401dded2b5b7cea9c2c0fb158fbd41a` rejected Account Security and Downloads tablet/mobile mappings because test-controlled viewport evidence was initially evaluated as if every viewport required a separate Playwright project.

The validator was corrected to distinguish exact `test-controlled` viewport markers from project-selected viewport evidence while preserving the blocking zero-retry Chromium requirement. It was not weakened.

The responsive suite then exposed hidden desktop language/navigation selectors on tablet/mobile. The tests were corrected to require visible language and `Aktualności` links through the real responsive navigation.

## Exact final validation

All authoritative final-head workflows passed on `f1524e767107afbf542d0d82e29a0f89eada1fc5`:

- Agent Governance `30534986330`;
- CI `30534986311`;
- Portal Acceptance Contract `30534986323`;
- Acceptance E2E and Visual UX `30534986337`;
- Downloads Acceptance `30534986351`;
- Phase 7 Production-Like Validation `30534986363`;
- Platform DB Outage Validation `30534986343`;
- Edge Security Emulation `30534986361`;
- Game Auth Ticket Concurrency `30534986329`.

Portal Acceptance passed strict route, product, backend/frontend and dimension closure plus the complete zero-retry account lifecycle. Visual UX passed Chromium smoke, Firefox/WebKit portability, responsive desktop/tablet/mobile, dependency resilience and keyboard accessibility.

## Claim boundary

- exact declared viewport/browser linkage tracked by #347 is closed;
- parent #326 remains open for unproven every-screen state/data/error/media permutations;
- Issue #350 / PR #351 remains the separate completed public game-data stress and real-500 slice;
- `PRODUCT_COMPLETE` remains false;
- exact deployed current-main staging was not proven by this task;
- `PRODUCTION_PROVEN` remains false;
- no runtime domain, schema, Canary, payment-provider, user-data or production mutation occurred.

## Next action

Continue parent #326 through a separately owned, bounded state/data/error/media evidence slice. Do not reinterpret this dimension contract as exhaustive frontend completeness.
