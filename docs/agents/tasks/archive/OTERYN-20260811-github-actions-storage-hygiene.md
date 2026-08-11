---
task_id: OTERYN-20260811-github-actions-storage-hygiene
status: completed
implementation_pull_request: 980
implementation_head: 8f6efc6b08f9c7b31160db5ec1ffdb032c83e9c2
implementation_merge_commit: 11e223f5f0883f0f3096769fbc2291de7edae62e
closeout_pull_request: 993
closeout_functional_head: 1fb43d31ce05a8c200ccb2e64f590277236a2c7c
closeout_final_head: 44f24e05617e9e1e480fef130f370e3a0b2a5466
closeout_merge_commit: 7edd8c9480f3faa8dbea49cc63542861a346295a
completed_at: 2026-08-11T10:31:00Z
---

# OTERYN-20260811 GitHub Actions storage hygiene — terminal archive

## Terminal outcome

Implementation PR #980 was squash-merged to `main` as `11e223f5f0883f0f3096769fbc2291de7edae62e`. Closeout PR #993 was subsequently squash-merged as `7edd8c9480f3faa8dbea49cc63542861a346295a` after its temporary one-time push authority had served its bounded bootstrap purpose.

Resulting `main` was verified at `7edd8c9480f3faa8dbea49cc63542861a346295a`. The permanent storage-hygiene workflow has no `push` trigger, retains exact closed-PR cache cleanup, runs bounded maintenance twice daily at `23 3,15 * * *`, and keeps manual cleanup confirmation-gated by `CLEAN_ACTIONS_STORAGE`.

## Bounded live result

Trusted-main workflow run `31476011425` completed two successful cleanup attempts under the 700-delete per-execution safety budget:

- attempt 1 job `93729781096`: 494 old artifacts / 4,244,404,516 bytes and 206 closed-PR caches / 2,273,009,806 bytes deleted;
- attempt 2 job `93733099349`: 531 old artifacts / 322,357,146 bytes and 169 closed-PR caches / 22,419,524 bytes deleted;
- combined: 1,400 exact resources, comprising 1,025 artifacts and 375 closed-PR caches;
- combined explicit reclaimed bytes: 6,862,190,992 bytes (~6.39 GiB);
- completed workflow runs deleted: 0, because the repository was younger than the 30-day run-retention threshold.

Attempt 2 post-state reported 14,523 artifacts and 906 active caches using 3,224,537,526 bytes. Another 4,344 qualifying historical candidates remained only because of the per-run deletion budget. They are an intentional provider-side maintenance backlog, not abandoned task-owned execution resources.

## Permanent safety and capacity contract

The terminal workflow preserves these boundaries:

- artifacts become cleanup candidates only after 14 days;
- completed workflow runs become candidates only after 30 days;
- closed pull-request cache cleanup targets only that PR's `refs/pull/<n>/merge` cache scope;
- each maintenance execution remains capped at 700 exact deletions and the script retains its API-rate reserve;
- twice-daily scheduling provides up to 1,400 bounded cleanup slots per day without increasing the authority of any single run;
- superseded `pull_request` validation generations may cancel, while closed-PR and maintenance cleanup executions are not cancelled by that policy;
- open-PR caches, branch/default-branch caches and recent evidence are preserved;
- packages, GHCR images, releases, repository content, environments, secrets and unrelated resources remain outside deletion authority.

The schedule was increased from weekly to twice daily during closeout because attempt-1 inventory observed 10,706 artifacts still inside the 14-day retention window, approximately 765 retained artifacts per day across that window. A weekly 700-delete capacity did not have demonstrated count headroom at that observed rate.

## Final validation

Implementation head `8f6efc6b08f9c7b31160db5ec1ffdb032c83e9c2` passed all emitted final exact-head workflows before #980 merged:

- Agent Governance `31474126909` — PASS;
- CI `31474126854` — PASS;
- GitHub Actions Storage Hygiene `31474126884` — PASS;
- Phase 7 Production-Like Validation `31474126847` — PASS;
- Platform DB Outage Validation `31474126899` — PASS;
- Game Auth Ticket Concurrency `31474126846` — PASS;
- Edge Security Emulation `31474126877` — PASS;
- Deep System Validation `31474126860` — PASS.

Closeout final head `44f24e05617e9e1e480fef130f370e3a0b2a5466` passed every repository-required exact-head check before #993 auto-merged:

- Agent Governance `31481150083` — PASS;
- GitHub Actions Storage Hygiene `31481150086` — PASS;
- Platform DB Outage Validation `31481150053` — PASS;
- Edge Security Emulation `31481150067` — PASS;
- Phase 7 Production-Like Validation `31481150101` — PASS;
- Game Auth Ticket Concurrency `31481150068` — PASS;
- CI `31481150045` — PASS.

Deep System Validation `31481150090` was not a repository-required merge gate for #993. At the terminal merge boundary it remained in progress after all setup, Composer, PHP/static, complete PHP regression/concurrency, browser advisory, strict portal-evidence and exact-SHA runtime-start stages had passed; the zero-retry browser matrix was the active stage. No Deep failure was present at merge.

## Review closeout

PR #980 repaired and resolved both Codex findings before merge: workflow-run search now partitions around GitHub's filtered 1,000-result ceiling, and artifact selection no longer suppresses an old artifact unless its exact parent run is selected.

PR #993 repaired and resolved two Codex P2 findings: the active checkpoint no longer carried a stale terminal merge action, and the authoritative checkpoint was refreshed to the final functional closeout revision. The final independent Codex review of `44f24e05617e9e1e480fef130f370e3a0b2a5466` reported no major issues. Whole-diff exact-head self-review was PASS and no requested-changes review or unresolved material thread remained.

Related lifecycle PR #990 was intentionally closed unmerged as superseded by #993, leaving one canonical closeout path.

## E2E and execution-resource hygiene

E2E for the closeout-only trigger/schedule/evidence change is `NOT_APPLICABLE`: it does not introduce a user-facing or new integration journey. The executable cleanup path itself was proven live on trusted `main` by successful run `31476011425` attempts 1 and 2.

The validation workflows use GitHub-hosted ephemeral runners. CI evidence showed runner-managed `Stop containers` completion, and this task created no retained external cloud, remote-host or self-hosted temporary execution resource requiring separate teardown. The 4,344 remaining provider-side eligible Actions objects are deliberately handed to the permanent bounded maintenance workflow under the retention/safety contract above.

## Issue and ownership reconciliation

Neither #980 nor #993 declares a closing implementation Issue; this task record is the canonical lifecycle owner. No unrelated Issue is closed as part of this archive. Repository/path ownership for this task is released by terminal archival.

## Provenance

The pre-archive active task record is preserved in Git history at blob `079fc4db1e7f6efecf3e08c944caa5ed8d07d0df`. Sanitized live execution evidence remains at `docs/agents/evidence/OTERYN-20260811-actions-storage-hygiene-live.md`.
