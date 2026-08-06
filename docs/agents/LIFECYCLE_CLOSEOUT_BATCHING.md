# Lifecycle Closeout Batching and Audit Artifact Contract

```yaml
lifecycle_closeout_batching_version: 1
repository: blakinio/Oteryn-Platform
applies_to:
  - OTERYN_PLATFORM_CONTINUOUS_AUDIT
  - OTERYN_PLATFORM_REMEDIATION
controlling_specialization_over:
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
objective: reduce governance-only PR, Issue and CI churn without weakening independent exact-head validation
```

## Controlling rule

Product, runtime, migration, contract, architecture, dependency, workflow, deployment and security changes remain bounded by one coherent root cause, one owned task, one branch and one implementation PR.

This contract creates a narrow exception only for **lifecycle-only** reconciliation after the underlying implementation work is already terminal. It also defines the default artifact for an independent audit.

## Independent audit artifact policy

A fresh independent audit normally produces evidence on the existing target PR:

1. inspect the exact final diff, acceptance evidence, environment outcome, checks and review state;
2. submit a PR review or top-level PR comment anchored to the exact audited SHA;
3. record `PASS_ZERO_MATERIAL_FINDINGS` or exact findings in the linked audit Issue or durable audit record;
4. close the audit Issue after the result is durable.

A PASS-only audit **must not create a new pull request**.

An audit PR is permitted only when the auditor is separately authorized to change audit documentation or evidence files that cannot be recorded accurately in a review, comment, Issue or existing task record. An audit must never modify the implementation branch it is validating.

A discovered defect returns to the owning implementation/remediation PR when ownership remains valid. If a separate root cause or ownership boundary is proven, create a separate remediation Issue and PR; do not disguise a fix as an audit PR.

## Eligible lifecycle-only closeout

A closeout item is eligible for batching only when every condition is proven:

- all underlying implementation, integration and remediation PRs are already intentionally terminal;
- the delivered scope and nonclaims are already supported by exact evidence;
- the remaining mutations are limited to task archival, programme or ledger reconciliation, Issue status evidence, ownership/lease release, branch terminal metadata and related PR classification;
- no application, frontend, migration, schema, API/protocol contract, architecture decision, dependency manifest/lockfile, generated contract/type, test behavior, workflow, deployment manifest, environment configuration or production state changes;
- no active product-code ownership or valid conflicting claim remains for the item;
- the items are independent and can be reviewed without hidden merge order or shared runtime effects.

If any condition is `UNKNOWN`, `CONFLICT`, or false, the item stays separate under the normal one-root-cause workflow.

## Batch shape

A lifecycle reconciliation wave uses:

```yaml
batch:
  size: 2..10
  task: one coordinator task
  branch: docs/lifecycle-closeout-wave-<YYYYMMDD>-<sequence>
  pull_request: one governance-only PR
  audit: one fresh independent exact-head audit with per-item verdicts
  ci: one required exact-head workflow generation for the batch PR
  follow_up_archive_pr: forbidden
```

The batch PR may move several completed task records from `active/` to `archive/`, reconcile their Issues and release ownership in the same diff. It must not create one follow-up archive PR per item.

The default one-Issue/one-branch/one-PR claim rule remains mandatory for product mutations. A lifecycle batch is a coordinator-owned governance task after product ownership is terminal; it is not a way to combine active implementation Issues.

## Batch inventory

The coordinator must include one independently reviewable record for every item:

```yaml
items:
  - task_id: <task>
    finding_or_issue: <number or id>
    implementation_prs:
      - <repo#number and terminal state>
    implementation_exact_heads:
      - <sha>
    implementation_merge_or_close_evidence:
      - <sha or exact terminal reason>
    delivered_scope: <bounded statement>
    preserved_nonclaims:
      - <statement>
    lifecycle_paths:
      - <exact active/archive/programme paths>
    ownership_release:
      owned_paths: []
      leases: []
    audit_verdict: pending | pass | finding
```

The PR description must list all included task IDs and Issues. A reviewer must be able to remove one item without changing the meaning of the others.

## Batch claim and collision prevention

Before creating a batch:

1. search active tasks, branches, PRs, Issues and claim comments for every candidate item;
2. exclude any item with a valid active implementation or closeout owner;
3. create one active coordinator task that owns the exact lifecycle paths for all included items;
4. publish the batch branch and PR before editing additional shared lifecycle indexes;
5. never reset, delete, supersede or absorb another agent's live branch merely to fill a batch.

Existing individual lifecycle-only PRs may be consolidated only by an explicit coordinator after verifying that no active session still owns them. Equivalent work must first exist on the batch branch; individual PRs are then closed accurately as superseded. Unique work must never be discarded.

## Independent batch audit

One fresh validator audits the entire exact batch SHA and records a verdict per item plus a whole-diff verdict.

```yaml
audit_result:
  exact_head: <sha>
  whole_diff: PASS_ZERO_MATERIAL_FINDINGS | FINDINGS
  per_item:
    - task_id: <task>
      result: PASS | FINDING
      evidence: <exact paths and source state>
  unresolved_review_threads: 0
```

The validator submits the result as a review/comment on the batch PR and may use one batch audit Issue when durable scheduling or claiming is needed. Do not create one audit Issue per item unless the items require different independent validators or security/authority boundaries.

If one item fails, remove or repair that item, rerun affected checks and re-audit the new exact head. Unaffected items may remain in the batch when their evidence is still coherent.

## Status and rotation semantics

Needing a fresh independent validator is a **role rotation**, not an external wait.

- The implementing checkpoint becomes `ready` with `next_action` naming the exact independent audit.
- The implementing invocation returns `ROTATE` when no fresh validator can run in that session and no other safe authorized work remains.
- Use `WAITING` only for a genuine external dependency, unavailable permission/environment, observation window, protected operation, owner decision or exhausted bounded terminal-CI wait.
- A fresh session may claim the audit and must remain read-only toward the implementation branch.

## Non-eligible work

Never batch:

- application or frontend behavior changes;
- authentication, authorization, payments or other security-sensitive fixes;
- migrations, shared schema changes or data repair;
- API, protocol, contract or generated-type changes;
- architecture decisions or canonical architecture rewrites;
- dependency or lockfile changes;
- CI workflow, deployment, runner or environment changes;
- changes with distinct rollback, release order or production risk;
- unresolved audit findings that require implementation.

These remain separate even when that creates more PRs.

## Closeout

A lifecycle batch is complete only when:

- every item has exact source evidence and an explicit verdict;
- the whole exact diff has a fresh independent PASS with zero material findings;
- required CI passes on the exact batch head;
- every included and superseded PR is intentionally terminal;
- all included task records are archived or otherwise terminal;
- Issues and programme state are reconciled;
- ownership and leases are released;
- no follow-up archive-only PR is required.
