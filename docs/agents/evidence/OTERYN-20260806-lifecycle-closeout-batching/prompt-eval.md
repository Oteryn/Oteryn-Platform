# Prompt evaluation — lifecycle closeout batching and audit artifacts

## Change under evaluation

Candidate adds `docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md` and binds it into the continuous-audit and remediation programme states plus the short-command registry.

Baseline: programme version 2 / registry 1.2.

Candidate: programme version 3 / registry 1.3 / lifecycle batching contract version 1.

## Evaluation method

Static adversarial contract review against representative routing, closeout, independence, collision and safety cases. The result is evaluated from the complete trusted routing set.

Repeated nondeterministic model trials: `NOT_RUN` because no repository harness capable of repeated programme-routing trials was identified. Exact-head governance CI remains mandatory.

## Cases

| # | Condition | Required result | Baseline | Candidate |
|---|---|---|---|---|
| 1 | Material authentication fix | One Issue, task, branch and implementation PR | PASS | PASS |
| 2 | Material migration fix | Separate PR with rollback and exact validation | PASS | PASS |
| 3 | PASS-only fresh audit | Review/comment on existing target PR; no audit PR | Ambiguous | PASS |
| 4 | Audit discovers defect in owned implementation | Return finding to implementation PR | PASS | PASS |
| 5 | Audit discovers separate root cause | Create separate remediation Issue/PR | PASS | PASS |
| 6 | Six completed tasks need only active→archive and ownership release | One bounded lifecycle wave PR | FAIL: default encouraged six PRs | PASS |
| 7 | Lifecycle batch contains product code | Reject batching | PASS | PASS |
| 8 | Lifecycle batch contains workflow or deployment change | Reject batching | PASS | PASS |
| 9 | Lifecycle batch contains 11 items | Split into bounded waves of at most 10 | Ambiguous | PASS |
| 10 | One item in batch fails audit | Remove/repair item and re-audit exact new head | Ambiguous | PASS |
| 11 | Existing lifecycle PR is actively owned by another session | Do not absorb or close it | PASS | PASS |
| 12 | Existing unowned lifecycle PR has unique work | Preserve equivalent work before closing superseded | Ambiguous | PASS |
| 13 | Implementer reaches mandatory independent audit gate | Checkpoint ready and return ROTATE, not WAITING | FAIL/ambiguous | PASS |
| 14 | External environment or permission is unavailable | Use WAITING with exact blocker | PASS | PASS |
| 15 | Batch audit scheduling requires durable claim | At most one batch audit Issue with per-item verdicts | Ambiguous | PASS |
| 16 | Security-sensitive closeouts appear documentation-only but alter risk claims | Keep separate when material security evidence or acceptance changes | Ambiguous | PASS |
| 17 | Batch PR merges | Archive, Issue reconciliation and ownership release occur in same PR; no archive follow-up PR | Ambiguous | PASS |
| 18 | Parallel workers see compatible lifecycle-only Issues | Route to coordinator batch, not one worker per item | FAIL/ambiguous | PASS |

## Outcome

Candidate result: **18/18 PASS**.

Baseline result: **8 PASS, 8 ambiguous, 2 FAIL**.

The candidate preserves independent validation and one-root-cause product isolation while removing unnecessary PASS-only audit PRs, per-task lifecycle PRs, duplicate archive PRs and repeated CI generations.

## Acceptance mapping

- Product/runtime root causes remain isolated: PASS.
- PASS-only audits use existing PR review/comment: PASS.
- Lifecycle-only waves are bounded to 2–10 compatible items: PASS.
- One exact-head batch audit records per-item verdicts: PASS.
- Active ownership cannot be stolen or reset: PASS.
- `ROTATE` versus `WAITING` semantics are explicit: PASS.
- Registry and both programme states require the contract: PASS.
- Runtime E2E: `NOT_APPLICABLE` because only governance routing changes.
