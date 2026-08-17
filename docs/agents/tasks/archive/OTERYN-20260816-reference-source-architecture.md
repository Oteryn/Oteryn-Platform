---
task_id: OTERYN-20260816-reference-source-architecture
repository: blakinio/Oteryn-Platform
mode: architecture
task_kind: discovery
issue: 1121
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-content
status: completed
closed_at: 2026-08-17T16:37:08+02:00
pr: 1122
validated_head: 43a41d7dbc84d45751976ba3b9b61b4fddef52a9
merge_sha: e168cdf0189b361ec299b040c0e825ec694bed97
---

# OTERYN-20260816-reference-source-architecture — COMPLETED

## Terminal result

Issue #1121 is completed. PR #1122 was transitioned from Draft to Ready under the repository's current central Spark policy and squash-merged into protected `main` as `e168cdf0189b361ec299b040c0e825ec694bed97`.

ADR 0042 and `docs/contracts/NON_NATIVE_REFERENCE_CONTENT_CONTRACT.md` establish a separate logical `ReferenceContent` boundary for provenance-pinned `NON_AUTHORITATIVE_REFERENCE` data. This resolves the source-authority dependency for the OTERYN_CONTENT_COMPLETION programme without weakening ADR 0034.

## Accepted invariants

- Oteryn-v2 remains the sole native executable gameplay-content authority.
- ReferenceContent never participates in GameCatalog authority-profile activation or fallback and never uses `legacy-canary`.
- Reference/source-local identity cannot mint native identity; crosswalks require independent target-authority evidence.
- Source Lua/code is never executed to derive reference facts.
- Reference snapshots do not prove runtime availability/reachability, authoritative absence/tombstones, or current native freshness.
- Native authority unavailability never promotes reference data to fallback truth.
- `data-global` and `data-crystal` remain separate source profiles.
- Source conflicts remain explicit; fields are not blended into an apparently authoritative entity.
- Wiki and PlayerCompanion preserve evidence class, snapshot/profile provenance, assumptions and limitations.
- Third-party prose/dialogue/maps/media publication rights remain separately fail-closed under ADR 0026 and `THIRD_PARTY_NOTICES.md`.

## Final exact-head validation

Validated implementation head: `43a41d7dbc84d45751976ba3b9b61b4fddef52a9`  
Protected base before merge: `a0b07e5362204f727e6713ca56d204269855ee5b`

```yaml
whole_diff_self_review:
  review: 4952334812
  result: PASS
  findings: []
agent_governance:
  run: 32039425895
  result: PASS
ci:
  run: 32039425815
  classify_changes: PASS
  runtime_tests: PASS
  required_test_gate: PASS
native_protocol_contract:
  run: 32039425712
  result: PASS
native_protocol_contract_audits:
  run: 32039425808
  result: PASS
platform_db_outage_validation:
  run: 32039425711
  result: PASS
game_auth_ticket_concurrency:
  run: 32039425778
  result: PASS
phase_7_production_like_validation:
  run: 32039425782
  result: PASS
review_threads_before_merge: 0
runtime_browser_e2e: NOT_APPLICABLE_DOCS_ONLY
```

Edge Security Emulation run `32039425735` first failed before application setup because Composer dependency downloads from `codeload.github.com` were rate-limited with HTTP 429. No application/edge assertion ran before that failure. One exact-job retry was started on unchanged head; Edge Security Emulation is not a protected-branch required context for this documentation-only architecture PR. The protected required contexts `classify-changes` and `test` both passed before merge.

## Spark readiness evidence

The earlier hosted-Codex Draft blocker became obsolete on 2026-08-17. Current root `AGENTS.md` grants standing authorization only to the central `blakinio/github-projects-control` controller using exactly `gpt-5.3-codex-spark` through ChatGPT-managed Codex authentication. `Oteryn-Platform` is in the active Spark allowlist; hosted Automatic Review/direct `@codex review`, Codex CLI, `OPENAI_API_KEY`, standard hosted Code Review and fallback models remain unauthorized.

PR #1122 therefore left Draft without invoking hosted Code Review. No owner-funded standard Codex/OpenAI API invocation was performed by this architecture task.

## Merge evidence

```yaml
pr: 1122
ready_before_merge: true
expected_head_merge_guard: 43a41d7dbc84d45751976ba3b9b61b4fddef52a9
merge_method: squash
merge_sha: e168cdf0189b361ec299b040c0e825ec694bed97
main_verified_after_merge: e168cdf0189b361ec299b040c0e825ec694bed97
issue_1121_state: closed_completed
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1122 merged successfully through the ordinary protected same-repository path.
source_branch_evidence: Live branch search after merge returned no `docs/issue-1121-reference-source-architecture` ref.
```

## CONTENT-COORD terminal handoff

ADR 0042 is canonical on protected `main`; the architecture question is resolved and no lane is `DECISION_REQUIRED`.

```text
SOURCE-PIPELINE: READY
  Bounded deterministic static extraction/normalization into ReferenceContent only.

WIKI-REFERENCE: READY
  Structured reference may consume NON_AUTHORITATIVE_REFERENCE with explicit provenance and limitations.
  Expressive third-party publication rights remain a separate fail-closed gate.

PLAYER-COMPANION: READY
  Reference-aware planning/simulation/recommendation slices are permitted.
  Current authoritative deterministic gameplay truth still requires authoritative GameCatalog/ruleset evidence.
```

## Authorization / nonclaims

No runtime/product implementation, database migration, deployment, production/staging/protected-environment mutation, external server/game repository access, hosted owner-funded Codex/OpenAI API use, or public copying of third-party expressive content was performed.

## Lifecycle closeout

This archive record replaces `docs/agents/tasks/active/OTERYN-20260816-reference-source-architecture.md`, releases the architecture task/path ownership, and hands the resolved dependency back to CONTENT-COORD for normal implementation dispatch.