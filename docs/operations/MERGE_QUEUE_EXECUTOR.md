# Merge Queue enqueue executor (META native `merge-async`)

## Purpose and authority

`.github/workflows/merge-queue-enqueue.yml` is the narrow repository-native path for submitting an already-qualified same-repository pull request to protected `main` Merge Queue. Its only positive mutation is:

`PUT /repos/Oteryn/Oteryn-Platform/pulls/{pr}/merge-async`

The JSON body is exactly the qualified `sha` and explicit `merge_action: merge_queue`. There is no direct merge, default-action, generic auto-merge, GraphQL, dequeue, protection-bypass, or check-mutation fallback.

## Authentication: no custom GitHub App

Do **not** create a dedicated Oteryn GitHub App, App client ID, or private key for this executor.

The workflow deliberately separates read authority from mutation authority:

- `MQ_GITHUB_READ_TOKEN=${{ github.token }}` uses the built-in read-only `GITHUB_TOKEN` for PR identity, exact-head `platform-gate` preflight, and fresh PR target readback;
- `MQ_GITHUB_MUTATION_TOKEN=${{ secrets.OTERYN_MQ_TOKEN }}` is a fine-grained personal access token used only for native `PUT .../merge-async` and `GET .../merge-async/{uuid}`.

`OTERYN_MQ_TOKEN` must be fine-grained, scoped only to the repository(s) that need autonomous Merge Queue submission, and grant **Contents: Read and write** only unless GitHub changes the endpoint contract. The workflow does not use that token to read checks or PR metadata. Never record or echo its value in Git, Issues, comments, logs, artifacts, or task evidence.

The built-in `GITHUB_TOKEN` must remain read-only and must not perform the queue mutation. GitHub suppresses most workflow runs caused by `GITHUB_TOKEN`; using it for queue admission could prevent the required independent `merge_group` workflow from being created. A fine-grained PAT is a supported credential for native `merge-async` and avoids any custom-App bootstrap.

If `OTERYN_MQ_TOKEN` is absent or the token lacks the native endpoint capability, the executor fails closed as `BLOCKED_CAPABILITY_UNAVAILABLE`. It never selects another merge primitive.

## Invocation and preflight

Two invocation surfaces are supported:

1. `workflow_dispatch` from `main`, with a positive `pr_number` and exact 40-hex `expected_head_sha`;
2. a PR-conversation comment whose complete body is exactly `/oteryn-mq-enqueue <40-hex-head>`, authored with association exactly `OWNER`, `MEMBER`, or `COLLABORATOR`. The PR number comes only from `github.event.issue.number`.

Non-PR issues, unauthorized associations, whitespace changes, malformed SHAs, and extra tokens do not start mutation. Both invocation paths reject before mutation unless live read-only state proves exact repository `Oteryn/Oteryn-Platform`, positive PR number, open/unmerged/non-draft state, same-repository head, `base=main`, exact qualified head, and latest exact-head `platform-gate=completed/success`.

## Acceptance receipt and causal readback

HTTP `202` means request acceptance only. The executor requires the server response to contain a canonical UUID, status, exact expected head, and explicit `merge_queue` action. It records those with repository, PR, base and a positive process-owned monotonic receipt sequence. The sequence is generated inside the executor and cannot be supplied by the workflow or caller.

Immediately afterward, the mutation client reads `GET /repos/{owner}/{repo}/pulls/{pr}/merge-async/{uuid}` while the read-only client freshly reads the PR target. The readback must bind the same UUID, repository and PR, `base=main`, unchanged exact head, and explicit `merge_queue`, with an executor sequence strictly greater than the receipt sequence. Missing/malformed/mismatched UUID, missing required server fields, equal/lower/boolean/nonpositive sequence, stale target, retarget, or head change fails closed. Whole-second timestamps, when present in surrounding GitHub evidence, are freshness evidence only and are never causal-order authority.

HTTP `200` and `409` are reconciliation-only. The executor performs the available async-status and PR identity readbacks, returns `RECONCILIATION_REQUIRED`, and never fabricates a new `202` receipt. HTTP `400` and `422` are request rejection. HTTP `403` and `404` are `BLOCKED_CAPABILITY_UNAVAILABLE`; permission/capability denial never selects another merge primitive.

## Completion boundary

A receipt, pending async state, or queue-admission event is non-terminal. Do not claim integration until a real `merge_group` aggregate `platform-gate` succeeds and protected `main` readback contains the integrated candidate. The executor never automatically dequeues an ambiguous request.

Historical bootstrap PR #1382 integrated the native `merge-async` implementation as protected `main@57b775a932ad24a7fad5d7c9120f725a4c451374`. Real canary PR #1383 then proved the exact comment trigger, authorization and exact-head qualification path but exposed the obsolete custom-App bootstrap before `merge-async` was called. The follow-up app-free repair must integrate normally, after which the same #1383 carrier is retried once to prove `202 + UUID + causal readback + merge_group + protected-main` end to end.

## Lifecycle

The workflow remains registered in `docs/agents/CI_WORKFLOW_LIFECYCLE.json`. Review or retire it if GitHub changes native async authentication/event semantics or if a protected replacement preserves every fail-closed and terminal-proof property above.
