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

Issue #1305 is the canonical lifecycle authority. Delete three terminal obsolete/validation branch refs through the existing reviewed exact-SHA protected-main workflow, without merging their content or weakening the controller.

## Acceptance criteria

- [x] Adoption #1304 is integrated and its temporary active claims are released.
- [x] Hosted PR inventory plus independently reconstructed live REST evidence proves the complete candidate set.
- [x] Canonical approval binds the reviewed candidate entries and current policy digest.
- [ ] Exact-head checks pass without controller or policy changes.
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
  - Exact-head hosted qualification is required before readiness.
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T10:19:00Z
head: 8fe46be5f4890bac333b377c183d50aa3cf2d35c
branch: governance/terminal-canary-cleanup-1305
pr: 1306
status: validating
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
  - PR #1270 is closed unmerged at exact current branch head c2edd9f98fdad7496c79c6c2534251549ce577c7 and was explicitly superseded by merged adoption #1304.
  - PR #1306 Terminal Branch Lifecycle run 34109541884 job 101702364318 inventoried 21 branches and exactly three eligible candidates after the temporary claims were released.
  - Live branch and head-filtered PR readback reconstructs exactly three canonical entries; their helper-computed digest is 1fe4618484ee29b7fd270c25c87961d2bc750c1ff3737b030b5eb25dfb0cd7df.
derived:
  - The #1050 identity and confirmation in the approval schema locate the existing mechanism; current mutation authority is #142/#1301/#1305 and does not reopen #1050.
unknown:
  - Exact-head hosted validation and protected-main apply result.
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
    result: PASS
    evidence: 7 cleanup, 8 guarded, 10 approval and 5 reusable tests pass locally; hosted phase-1 validation tests and base policy also passed.
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: Repository branch metadata cleanup only; no application, auth, data, payment or deployment behavior changes.
blockers:
  - Exact-head hosted qualification is required before readiness.
next_action: Publish the independently reviewed exact three-entry approval, then qualify the exact PR head.
```

## Authorized targets

| Branch | Exact head | Closed PR | Recovery |
|---|---|---:|---|
| governance/agent-stall-loop-prevention | `c2edd9f98fdad7496c79c6c2534251549ce577c7` | #1270 | immutable commit and PR; superseded `AGENTS.md` proposal preserved without merge |
| test/codex-native-publish-canary-20260903-platform | `d866d21ca856b925a32a3a5457174906237db199` | #1290 | immutable commit and PR; only `CODEX_NATIVE_PUBLISH_CANARY.md` |
| test/codex-shared-pat-canary-20260903-platform | `65126f2b1d24e0fe497e436be99f524104021033` | #1291 | immutable commit and PR; only `CODEX_SHARED_PAT_CANARY.md` |

The table intentionally does not create `lock_branch` fields: active claims make a candidate ineligible, and this task's purpose is a reviewed deletion. The phase-1 provisional approval had `apply_on_main: false` and an impossible digest, so it could not authorize deletion. Its expected validation failure still produced hosted inventory evidence in the job log (artifact upload was skipped after failure); final approval uses separately reconstructed live REST identities plus the hosted candidate count and receives independent review before readiness.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Dedicated same-repository cleanup branch; ordinary delete-after-merge applies after guarded target deletion.
source_branch_evidence: Pending protected merge, workflow apply evidence and source-ref readback.
```
