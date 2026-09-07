---
task_id: OTERYN-20260907-terminal-canary-cleanup
governing_issue: 1305
required_reads:
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - .github/workflows/terminal-branch-lifecycle.yml
search_first:
  - tools/agents/terminal_branch_cleanup.py
  - tools/agents/terminal_branch_approval.py
optional_reads: []
---

# Terminal validation canary cleanup

## Goal

Issue #1305 is the canonical lifecycle authority. Delete two terminal validation-only branch refs through the existing reviewed exact-SHA protected-main workflow, without merging their content or weakening the controller.

## Acceptance criteria

- [x] Adoption #1304 is integrated and its temporary active claims are released.
- [ ] A PR dry run produces the complete live terminal candidate manifest.
- [ ] Canonical approval binds the reviewed candidate entries and current policy digest.
- [ ] Independent review and exact-head checks pass without controller or policy changes.
- [ ] Protected-main apply evidence proves guarded deletion and live absence.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260907-meta-agent-policy-v3-adoption.md
  - docs/agents/tasks/active/OTERYN-20260907-terminal-canary-cleanup.md
  - docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json
modules:
  - agent-governance
dependencies:
  - Oteryn/Oteryn#142
  - Oteryn/Oteryn-Platform#1301
  - Oteryn/Oteryn-Platform#1050 schema and mechanism provenance only
blockers:
  - Live PR dry-run manifest is required before final approval can be authored.
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T10:02:00Z
head: 907546f193e91b0bed2f5f077ab5b874771929ef
branch: governance/terminal-canary-cleanup-1305
pr: none
status: implementing
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260907-meta-agent-policy-v3-adoption.md
  - docs/agents/tasks/active/OTERYN-20260907-terminal-canary-cleanup.md
  - docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json
proven:
  - PR #1290 is closed unmerged and its must-not-merge canary ref exists at d866d21ca856b925a32a3a5457174906237db199.
  - PR #1291 is closed unmerged and its must-not-merge canary ref exists at 65126f2b1d24e0fe497e436be99f524104021033.
  - Protected main 907546f193e91b0bed2f5f077ab5b874771929ef contains the reviewed terminal lifecycle; its main-push apply path revalidates exact SHA, closed PR identity, active claims, protection, retention, reserved names, policy and approval digest before contents-write deletion.
derived:
  - The #1050 identity and confirmation in the approval schema locate the existing mechanism; current mutation authority is #142/#1301/#1305 and does not reopen #1050.
unknown:
  - Complete live terminal candidate entries and digest after temporary claims are released.
conflicts: []
first_failure:
  marker: historical-branch-audit-two-unexplained
  evidence: Run 34104877215 job 101687493437 artifact 10011942363 reports both exact canary refs.
rejected_hypotheses:
  - Merging or repointing either canary is not an acceptable deletion substitute.
  - Changing controller eligibility to force deletion is not authorized.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260907-meta-agent-policy-v3-adoption.md -> docs/agents/tasks/archive/OTERYN-20260907-meta-agent-policy-v3-adoption.md
  - docs/agents/tasks/active/OTERYN-20260907-terminal-canary-cleanup.md
  - docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json
validation:
  - command: python tools/agents/test_terminal_branch_cleanup.py && python tools/agents/test_terminal_branch_guarded.py && python tools/agents/test_terminal_branch_approval.py && python tools/agents/test_terminal_branch_reusable.py
    result: NOT_RUN
    evidence: Run after provisional dry-run packet is materialized.
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: Repository branch metadata cleanup only; no application, auth, data, payment or deployment behavior changes.
blockers:
  - Live PR dry-run manifest is required before final approval can be authored.
next_action: Publish a draft PR with a fail-safe non-applying provisional approval and capture its live terminal manifest artifact.
```

## Authorized targets

| Branch | Exact head | Closed PR | Recovery |
|---|---|---:|---|
| test/codex-native-publish-canary-20260903-platform | `d866d21ca856b925a32a3a5457174906237db199` | #1290 | immutable commit and PR; only `CODEX_NATIVE_PUBLISH_CANARY.md` |
| test/codex-shared-pat-canary-20260903-platform | `65126f2b1d24e0fe497e436be99f524104021033` | #1291 | immutable commit and PR; only `CODEX_SHARED_PAT_CANARY.md` |

The table intentionally does not create `lock_branch` fields: active claims make a candidate ineligible, and this task's purpose is a reviewed deletion. The provisional approval has `apply_on_main: false` and an impossible digest, so it cannot authorize deletion if merged accidentally; it exists only to trigger a PR dry-run artifact and must be replaced before readiness.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Dedicated same-repository cleanup branch; ordinary delete-after-merge applies after guarded target deletion.
source_branch_evidence: Pending protected merge, workflow apply evidence and source-ref readback.
```
