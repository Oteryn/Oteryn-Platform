# Merge Queue enqueue executor

## Purpose

`.github/workflows/merge-queue-enqueue.yml` is the repository-native operator path for adding an already-qualified same-repository pull request to the protected `main` Merge Queue. It exists so normal integration does not depend on an online maintainer workstation.

The workflow is intentionally manual-only and fail-closed. It does not call a REST merge endpoint, does not bypass branch protection, and does not use ordinary auto-merge as a substitute for Merge Queue.

## One-time GitHub App configuration

Create or reuse a dedicated GitHub App whose installation is limited to `Oteryn/Oteryn-Platform`. Grant only the repository permissions required by this executor:

- **Checks: Read** — read the exact-head `platform-gate` check run.
- **Pull requests: Read and write** — enqueue the qualified pull request through GraphQL.
- **Metadata: Read** — implicit GitHub App repository metadata access.

Install the App only on `Oteryn/Oteryn-Platform`, then configure these repository Actions values without committing their contents:

- repository variable `OTERYN_MQ_APP_CLIENT_ID` = the GitHub App client ID;
- repository secret `OTERYN_MQ_APP_PRIVATE_KEY` = the App private key.

The workflow uses pinned `actions/create-github-app-token` to mint a short-lived installation token scoped to the current repository and explicitly requests only `checks: read` and `pull-requests: write`. The action revokes the token in its post step.

## Invocation

Run **Merge Queue Enqueue** from the `main` workflow ref and provide:

- `pr_number`: the open pull request number;
- `expected_head_sha`: the exact 40-character head SHA whose required `platform-gate` already passed.

The executor rejects the request before mutation unless all of these are true:

1. repository is exactly `Oteryn/Oteryn-Platform`;
2. PR is open, unmerged, non-draft, and based on `main`;
3. PR head repository is the same repository;
4. current PR head SHA exactly equals `expected_head_sha`;
5. the latest matching `platform-gate` check run on that exact SHA is completed successfully.

Only after those checks does the script call GraphQL `enqueuePullRequest` with `expectedHeadOid` set to the same SHA. The server-side head fence therefore remains authoritative if the PR changes between qualification and enqueue.

## Failure behavior

Any input mismatch, missing/failed gate, changed PR head, GitHub API error, GraphQL error, missing queue entry, or missing App configuration makes the workflow fail without attempting an alternate merge path. Re-run only after refreshing the PR head and exact-head qualification evidence.

## Security boundary

The App credential is never written to Git, workflow output, issue text, or task records. `GITHUB_TOKEN` remains read-only for repository contents in this workflow; the App token is provided only to the enqueue script. The workflow job runs only when the dispatch ref is `refs/heads/main`.

The executor does not approve PRs, alter checks, change rulesets, modify branch protection, jump the queue, or merge directly.

## Lifecycle

This workflow has a distinct durable permission and operator lifecycle, so it is registered as manual-only in `docs/agents/CI_WORKFLOW_LIFECYCLE.json`.

Review or retire it when either:

- the connected automation surface exposes a native exact-head-fenced Merge Queue enqueue operation with equivalent fail-closed behavior; or
- GitHub changes Merge Queue or App-token semantics such that the current permission/fencing contract is no longer correct.

A future replacement must preserve normal Merge Queue authority and exact-head fencing before this workflow is removed.