# Organization Terminal Branch Lifecycle Design

**Date:** 2026-08-23
**Issue:** #1230

## Goal

Extend the existing fail-closed Platform terminal branch lifecycle into an organization capability for `Oteryn/Oteryn`, `Oteryn/Oteryn-Game`, and `Oteryn/Oteryn-Atlas` without creating an organization-wide destructive token.

## Architecture

`Oteryn/Oteryn-Platform` remains the implementation authority for the tested lifecycle scripts and exposes one reusable GitHub Actions workflow. Each caller repository owns a thin workflow plus a local policy/ADR. The caller pins the reusable workflow and tool checkout to one immutable merged Platform SHA.

The reusable workflow operates only on `github.repository` and uses the caller repository's `GITHUB_TOKEN`. The Platform tool checkout is read-only and uses `persist-credentials: false`; destructive git operations therefore retain the caller repository as `origin` and cannot gain write authority to Platform or another product repository.

## Operations and permissions

The reusable workflow exposes three explicit operations:

- `read`: validate caller policy and build a live fail-closed inventory; read-only permissions only.
- `close`: on a trusted same-repository `pull_request_target` close event, process an explicit `Branch-Disposition: delete|retain`; `contents: write` is granted only to this call.
- `apply`: after a reviewed historical approval is merged to caller `main`, rebuild the live candidate set, validate the exact reviewed digest, and apply exact-head deletions; `contents: write` is granted only to this call.

Caller wrappers schedule `read` daily, run `read` for lifecycle-policy pull requests/manual dispatch, run `close` for closed PR events, and run `apply` only on relevant `main` pushes.

## Safety invariants

Deletion remains exact-head and fail-closed. Default/protected branches, open PRs, active task claims, recovery/release/rollback/backup-sensitive refs, explicit retention exceptions, moved SHA/PR identity, forks, and ambiguous historical refs are never deleted by heuristic age or prefix rules.

No workflow checks out or executes untrusted PR code with write authority. `pull_request_target` cleanup checks out trusted protected `main`; the closed PR body is treated as data and still requires live same-repository branch/SHA/PR revalidation before deletion.

## Caller contract

Every caller repository provides:

- `.github/workflows/terminal-branch-lifecycle.yml` pinned to the merged Platform implementation SHA;
- `docs/agents/BRANCH_LIFECYCLE_POLICY.json` with `main` as the protected retention exception;
- a local accepted branch-lifecycle ADR referenced by that policy.

Repositories may add an optional `docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json` only for a separately reviewed historical cleanup set.

## Verification

Platform tests pin the reusable workflow's permission, pinning, checkout, operation and exact-root contracts. Existing branch lifecycle unit/negative tests remain mandatory. Each caller PR must parse/inspect the thin workflow, validate its local policy with the exact pinned Platform implementation, run its normal required CI, and remain non-runtime/non-deployment work.

## Rollout order

1. Merge the reusable workflow and contract to Platform.
2. Pin META, Game and Atlas callers to that exact merged SHA.
3. Merge each caller only after its own exact-head gates pass.
4. Run caller read-only inventories. Historical deletion remains separately reviewed; the rollout itself does not delete ambiguous old refs.
