# Lifecycle Closeout Batching and Audit Artifact Contract

```yaml
lifecycle_closeout_batching_version: 2
repository: blakinio/Oteryn-Platform
applies_to:
  - OTERYN_PLATFORM_CONTINUOUS_AUDIT
  - OTERYN_PLATFORM_REMEDIATION
repair_delivery_contract: docs/agents/REPAIR_PR_ECONOMY.md
controlling_specialization_over:
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
objective: reduce terminal governance-only PR and audit churn without weakening independent exact-head validation
```

## Controlling rule

This contract governs **terminal lifecycle-only reconciliation**. Active implementation delivery and repair trains are governed by `REPAIR_PR_ECONOMY.md`.

The mechanisms are distinct:

- **repair train** — compatible active implementations delivered in one bounded PR;
- **audit artifact** — review/comment on the exact target delivery PR;
- **lifecycle batch** — terminal governance-only reconciliation after underlying implementation is intentionally terminal.

Never use a lifecycle batch to combine active product, runtime, migration, contract, architecture, dependency, workflow, deployment or security changes.

## Independent audit artifact policy

A fresh audit normally:

1. inspects exact PR number, base SHA, head SHA, effective diff, acceptance, environment result, checks and review state;
2. is performed by an eligible distinct `AUDIT ONLY` agent/session that did not write or remediate target commits;
3. records whole-diff and per-item/Issue verdicts;
4. submits a review or top-level comment on the existing target PR;
5. records `PASS_ZERO_MATERIAL_FINDINGS` or exact findings in the linked audit Issue/record.

A PASS-only audit must not create a PR. Automated checks, self-review, implementer summaries and coordinator approval never substitute for the required independent audit.

Any effective target change invalidates the audit generation. Findings return to the implementation/closeout owner; an auditor that writes a fix becomes an implementer and cannot issue final PASS for that generation.

An audit PR is permitted only when separately authorized audit documentation/evidence cannot be recorded accurately in the existing PR, Issue or task. It must not mutate the audited implementation branch.

## Eligible lifecycle-only item

All conditions must be proven:

- underlying implementation/integration/remediation PRs are intentionally terminal;
- delivered scope and nonclaims have exact evidence;
- remaining changes are limited to task archival, programme/ledger reconciliation, Issue status evidence, ownership/lease release, branch terminal metadata and related-PR classification;
- no application, frontend, migration, schema, API/protocol, architecture, dependency/lockfile, generated contract/type, test behavior, workflow, deployment, environment or production change;
- no valid active implementation/closeout ownership conflict;
- items are independent and removable without hidden runtime or merge-order effects.

`UNKNOWN`, `CONFLICT` or false keeps the item outside the batch.

## Batch shape

```yaml
batch:
  size: 2..10
  task: one coordinator task
  branch: docs/lifecycle-closeout-wave-<YYYYMMDD>-<sequence>
  pull_request: one governance-only PR
  audit: one eligible independent exact-head audit with whole-diff and per-item verdicts
  ci: one required exact-head workflow generation
  follow_up_archive_pr: forbidden
```

Do not delay a ready closeout indefinitely merely to reach batch size 2. When no compatible terminal item is already available and same-PR closeout is impossible, use the smallest repository-safe terminal path and record why batching did not apply.

## Item inventory

```yaml
items:
  - task_id: <task>
    finding_or_issue: <number or id>
    implementation_prs:
      - <repo#number and terminal state>
    implementation_exact_heads:
      - <sha>
    implementation_merge_or_close_evidence:
      - <sha or exact reason>
    delivered_scope: <bounded statement>
    preserved_nonclaims:
      - <statement>
    lifecycle_paths:
      - <path>
    ownership_release:
      owned_paths: []
      leases: []
    audit_verdict: pending | pass | finding
```

The PR description lists all task IDs and Issues. Removing one item must not alter the meaning of the others.

## Batch claim and collision prevention

Before creating a batch:

1. search tasks, branches, PRs, Issues and claim comments for every item;
2. exclude any item with a valid owner;
3. create one coordinator task owning exact lifecycle paths;
4. publish branch and PR before editing further shared indexes;
5. never reset, delete, supersede or absorb live work to fill a batch.

Existing lifecycle-only PRs may be consolidated only after proving no active session owns them and preserving all unique work before accurate superseded closure.

## Independent batch audit

```yaml
audit_result:
  generation: <integer>
  auditor:
    session_or_claim: <identity>
    mode: AUDIT_ONLY
    wrote_target_commits: false
    mutated_target_branch: false
  target:
    pull_request: <number>
    base_sha: <sha>
    head_sha: <sha>
  whole_diff: PASS_ZERO_MATERIAL_FINDINGS | FINDINGS | PENDING
  per_item:
    - task_id: <task>
      result: PASS | FINDING | PENDING
      evidence: <exact evidence>
  material_findings_open: <integer>
  unresolved_review_threads: 0
```

PASS requires whole-diff PASS, every per-item result PASS and zero material findings. If one item fails, remove or repair it, rerun affected checks and audit the new exact head. Unaffected items may remain only when evidence remains coherent.

## Same-PR archival and completed-on-merge

Prefer archival and final governance reconciliation in the implementation/delivery PR when technically safe. Before merge, archival must not claim already-completed work.

Use:

```yaml
archive_state:
  status: completed_on_merge
  effective_when:
    pull_request: <number>
    exact_head: <sha>
    merged: true
```

Rules:

- `completed_on_merge` is conditional, not terminal completion;
- closing the PR without merge leaves the task active/ready/waiting/blocked and ownership retained or explicitly recovered;
- a changed exact head requires archive condition refresh;
- after successful merge, Issue closure/release comments may finalize external state without another PR;
- unavoidable post-merge repository housekeeping is consolidated at a programme barrier, not one PR per Issue.

## Status and rotation

Needing a distinct validator is role rotation, not external waiting.

- implementing checkpoint becomes `ready` with exact audit `next_action`;
- implementation invocation returns `ROTATE` if no eligible auditor can run in that session;
- `WAITING` is reserved for a real external dependency, accepted external actor, permission/environment, observation window, protected operation, owner decision or exhausted terminal-CI limit;
- a fresh audit session remains read-only toward the target branch.

## Non-eligible work

Never lifecycle-batch:

- active application/frontend behavior changes;
- authentication, authorization, payment or security fixes;
- migrations, shared schema changes or data repair;
- API/protocol/contract/generated-type changes;
- architecture decisions or canonical architecture rewrites;
- dependency/lockfile changes;
- CI workflow, deployment, runner or environment changes;
- distinct rollback, release order, observation or production risk;
- unresolved findings requiring implementation.

## Closeout

A lifecycle batch completes only when:

- every item has exact terminal source evidence and explicit PASS;
- whole exact diff has eligible independent PASS;
- required CI passes on the exact head;
- all included/superseded PRs are intentional terminal states;
- task records are archived/terminal;
- Issues/programme state are reconciled;
- ownership and leases are released;
- no follow-up archive-only PR is required.
