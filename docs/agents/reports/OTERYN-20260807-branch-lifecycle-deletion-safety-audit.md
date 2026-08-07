# Branch lifecycle destructive-apply safety audit — 2026-08-07

## Scope

Audit target: `blakinio/Oteryn-Platform@021bf44d99de4430b2e054d25872eabfa322eba2`.

Bounded domain: the accepted ADR 0024 branch-retention policy and its destructive apply implementation in `.github/workflows/branch-lifecycle.yml` and `tools/agents/branch_lifecycle.py`.

The audit is documentation/evidence only. It did not alter lifecycle code, tests, workflows, repository settings, staging, production or external repositories.

## Selection rationale

The continuous-audit baseline recorded `latest_audited_main=7319723520f3ee61e7dccc421742817253fdcfb9`; current main was 75 commits ahead. The native-protocol lane had just completed and Issue #558 had a fresh implementation claim, so those owned surfaces were excluded. Branch lifecycle was selected because it is recently introduced, unowned, and can destructively delete Git refs.

## Primary evidence

ADR 0024 requires deletion to fail closed for protected, open-PR, active-claim, release/rollback/recovery, unknown or conflicting branches.

`fetch_live_snapshot()` obtains branch SHA/protection, pull requests, active task branch references and deterministic remediation Issue state once. `classify_snapshot()` and `validate_manifest()` operate on that snapshot.

`apply_manifest()` then validates the full manifest against the in-memory report and loops over entries. For each entry it calls `client.delete_branch(entry["branch"])`, verifies only that the ref is absent afterward, and proceeds. The deletion API call is keyed only by branch name.

There is no live per-entry check between manifest validation and deletion that confirms:

- the branch ref still points to the reviewed SHA;
- no PR has just been opened;
- no task/claim has just activated the branch;
- protection/retention state has not changed.

The workflow's concurrency group serializes Branch Lifecycle workflow runs for the same ref, but it does not lock candidate branches against ordinary pushes, PR creation, task activation or Issue claims.

## Material finding

### OPA-GOV-0019 — HIGH — PROVEN

A candidate can be `TERMINAL_MERGED` at inventory time, then receive a new commit or become active before its turn in the deletion loop. The implementation still deletes the branch by name because its safety decision is based on the earlier snapshot.

Expected: destructive deletion is guarded by current live state at the moment of deletion and fails closed on any changed SHA, ownership, PR or protection evidence.

Actual: the apply path has a time-of-check/time-of-use gap across the full candidate loop.

Impact: newly active/unmerged branch work can have its ref deleted. Immutable commits may remain recoverable, but the accepted no-active-work-deletion invariant is violated and coordination/data-loss risk is introduced.

Durable remediation owner: Issue #780.

## Duplicate search

Open/closed Issues were searched for branch-deletion races, stale apply manifests, pre-delete SHA revalidation, active-claim races and TOCTOU. No duplicate actionable root cause was found. Issue #658 is the completed original implementation. Issue #558 owns a different Agent Governance liveness surface.

## Rejected hypotheses

- Reviewed manifest hashing closes the race: rejected. It proves the inventory candidate set did not drift while rebuilding the manifest, not that each ref remains unchanged immediately before its DELETE.
- Workflow concurrency closes the race: rejected. It prevents overlapping Branch Lifecycle runs, not unrelated branch/PR/claim activity.
- Post-delete absence verification closes the race: rejected. It confirms deletion occurred but cannot prove that the deleted ref was still the reviewed ref.

## Validation boundary

No destructive live race was intentionally manufactured. Primary code evidence is sufficient to prove the missing guard. Remediation acceptance requires deterministic offline/fake-client tests for SHA change, newly opened PR, newly active task/claim and protection/retention changes between validation and deletion.

Runtime E2E for this audit deliverable: `NOT_APPLICABLE`; no executable product behavior is changed by the audit record.
