# Merge Queue enqueue executor (META 3.1)

## Purpose and authority

`.github/workflows/merge-queue-enqueue.yml` is the narrow repository-native path for submitting an already-qualified same-repository pull request to the protected `main` Merge Queue. Its only positive mutation is:

`PUT /repos/Oteryn/Oteryn-Platform/pulls/{pr}/merge-async`

The JSON body is exactly the qualified `sha` and explicit `merge_action: merge_queue`. The executor has no direct merge, default-action, generic auto-merge, GraphQL, dequeue, protection-bypass, or check-mutation fallback. This implements the immutable META 3.1 policy bound at `Oteryn/Oteryn@ed6c8c98605a7fbfea858e0ef616f89baa617262`.

## One-time GitHub App bootstrap

Create or reuse a dedicated GitHub App installed only on `Oteryn/Oteryn-Platform`, with these repository permissions (plus implicit metadata):

- **Contents: Write** — invoke native `merge-async` with `merge_action: merge_queue`;
- **Checks: Read** — read the exact-head `platform-gate` check run;
- **Pull requests: Read** — perform fresh preflight and post-request target readback.

Configure repository variable `OTERYN_MQ_APP_CLIENT_ID` and repository secret `OTERYN_MQ_APP_PRIVATE_KEY`. Never record either credential value in Git, Issues, comments, logs, or task evidence. The pinned `actions/create-github-app-token` step requests only `permission-contents: write`, `permission-checks: read`, and `permission-pull-requests: read`, and revokes its short-lived installation token in the action post-step. Top-level `GITHUB_TOKEN` permissions remain read-only; it is not used for mutation because its event suppression would prevent the required independent `merge_group` workflow run.

## Invocation and preflight

Two invocation surfaces are supported:

1. `workflow_dispatch` from `main`, with a positive `pr_number` and exact 40-hex `expected_head_sha`;
2. a PR-conversation comment whose complete body is exactly `/oteryn-mq-enqueue <40-hex-head>`, authored with association exactly `OWNER`, `MEMBER`, or `COLLABORATOR`. The PR number comes only from `github.event.issue.number`.

Non-PR issues, unauthorized associations, whitespace changes, malformed SHAs, and extra tokens do not start the mutation job. Both invocation paths then reject before mutation unless live state proves exact repository `Oteryn/Oteryn-Platform`, positive PR number, open/unmerged/non-draft state, same-repository head, `base=main`, exact qualified head, and the latest exact-head `platform-gate` is `completed/success`.

## Acceptance receipt and causal readback

HTTP `202` means request acceptance only. The executor requires the server response to contain a canonical UUID, status, exact expected head, and explicit `merge_queue` action. It records those with repository, PR, base and a positive process-owned monotonic receipt sequence. The sequence is generated inside the executor and cannot be supplied by the workflow or caller.

Immediately afterward, the executor reads `GET /repos/{owner}/{repo}/pulls/{pr}/merge-async/{uuid}` and then freshly reads the PR target. The readback must bind the same UUID, repository and PR, `base=main`, unchanged exact head, and explicit `merge_queue`, with an executor sequence strictly greater than the receipt sequence. Missing/malformed/mismatched UUID, missing fields, equal/lower/boolean/nonpositive sequence, stale target, retarget, or head change fails closed. Timestamps may be supplemental bounded-freshness evidence only; they are not causal ordering.

HTTP `200` and `409` are reconciliation-only. The executor performs the available live async and PR readbacks, reports `RECONCILIATION_REQUIRED`, and never fabricates a `202` acceptance receipt. HTTP `400` and `422` are precise request rejection. HTTP `403` and `404` are `BLOCKED_CAPABILITY_UNAVAILABLE`; capability or permission denial never selects another merge primitive.

## Completion boundary

A receipt, pending async result, or queue admission is non-terminal. Do not close the task or claim integration until a later real `merge_group` aggregate `platform-gate` succeeds and protected `main` readback contains the integrated candidate. The executor never automatically dequeues an ambiguous request.

PR #1382 on `fix/1363-native-merge-async-executor` is the bootstrap repair. It must remain Draft until its exact-head checks are green and must integrate through the existing protected Merge Queue route. After this implementation reaches protected `main`, a separate qualified provider PR must supply the remaining real native canary evidence: receipt UUID/sequence, strictly later readback, real `merge_group` gate, and protected-main integration readback.

## Lifecycle

The workflow is registered in `docs/agents/CI_WORKFLOW_LIFECYCLE.json`. It is no longer manual-only because its exact authorized PR-comment connector trigger supplements `workflow_dispatch`. Review or retire it if GitHub changes the native async contract, App-token permissions, event behavior, or if a protected replacement preserves every fail-closed and terminal-proof property above.
