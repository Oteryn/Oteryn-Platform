# Repair Pull Request Economy and Repair-Train Contract

```yaml
repair_pr_economy_version: 1
repository: blakinio/Oteryn-Platform
applies_to:
  - OTERYN_PLATFORM_REMEDIATION
controlling_specialization_over:
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
objective: minimize unnecessary Pull Requests without weakening atomic ownership, independent validation, exact-head evidence, rollback or closeout
```

## Controlling principles

1. The deterministic branch `repair/issue-<ISSUE_NUMBER>` remains the atomic Issue lock. Labels, assignees, comments, task files and coordinator intent are visibility or routing evidence, not replacements for Git-ref arbitration.
2. Every Issue retains its own claim nonce, active task, exact ownership, source branch, acceptance criteria, validation and closure mapping.
3. Claim activation and PR creation are separate lifecycle events. A worker must not create a PR solely to prove activity.
4. Reuse an authoritative existing PR before creating another PR.
5. Compatible independently claimed repairs may use one bounded repair-train delivery PR.
6. Audit evidence, CI evidence, E2E evidence, Issue comments, ownership release and ordinary post-merge reconciliation do not require separate PRs.
7. Fewer PRs is not success when it creates hidden coupling, weaker review, unclear causality, migration/security risk or an unsafe rollback blast radius.

## Claim activation without a mandatory PR

A winning claim is active when all of the following agree:

- deterministic branch `repair/issue-<number>` exists and was acquired under `REMEDIATION_WORK_CLAIM_PROTOCOL.md`;
- machine-readable Issue activation identifies the claim nonce, task, branch, exact head, paths, lease and coordination key;
- an active task record exists on the deterministic branch;
- ownership and recovery state are visible and non-conflicting.

A PR is optional at activation. The activation record uses `pull_request: none | <number>` and `delivery_state: branch_only | reused_existing | dedicated_pr | train_candidate | repair_train`.

Open or reuse a PR early when at least one applies:

- PR-triggered CI is required;
- a reviewer or integration owner needs the incomplete diff;
- the change touches a high-risk boundary where early review is valuable;
- an authoritative existing PR must be reused;
- a coherent reviewable candidate exists;
- an accepted repair train has reached integration.

## Mandatory delivery selection order

Before creating a PR, query open and closed related PRs and use this order:

1. **Reuse authoritative existing PR** when its owner, purpose, branch and acceptance remain compatible.
2. **Join an open compatible repair train** through its sole integration owner and exact-source-head handoff.
3. **Continue branch-only implementation** on `repair/issue-<number>` until a coherent reviewable candidate exists.
4. **Create one dedicated delivery PR** only when reuse and safe train integration do not apply.

A replacement PR does not make an earlier PR terminal. Close duplicates and superseded attempts accurately before completion.

## Dedicated-PR boundaries

Keep a repair dedicated when it includes any of:

- P0 or urgent security work;
- authentication, authorization, session, credential or protected-data boundaries;
- payment or financial mutation;
- production deployment or protected-environment behavior;
- database migration head, schema authority or destructive transition;
- public API/protocol authority or generated-contract migration;
- dependency manifest, lockfile or supply-chain update unless an existing automated PR is reused;
- CI workflow or branch-protection semantics;
- architecture decision with an independent lifecycle;
- missing-module bootstrap or large feature slice;
- independent rollout, rollback, observation window or release order;
- conflicting ownership, coordination key or review audience;
- a combined diff that materially obscures causality, acceptance or rollback.

## Repair trains

A repair train is one delivery PR that imports exact accepted commits from multiple independently claimed Issue branches.

Default train size is 2–3 Issues. More than 3 requires recorded coordinator justification. Never keep a coherent completed repair waiting merely to fill a train; if no compatible train or second coherent candidate is already available, use a dedicated delivery PR.

A train is eligible only when:

- Issues belong to the same bounded programme wave or delivery area;
- source ownership and paths are non-overlapping;
- coordination keys are distinct;
- changes are compatible on the same current `main`;
- review and rollback remain understandable;
- no item requires a dedicated boundary listed above;
- combined exact-head audit, E2E and CI can safely validate the result;
- whole-train revert is acceptable or every Issue has independently reconstructable commit/path boundaries.

Exactly one integration owner writes the train branch. Workers never share the train branch.

Suggested branch form:

```text
repair-train/<area>-<wave>-<YYYYMMDD>
```

### Exact-source-head acceptance

Before import, record:

```yaml
repair_train_acceptance:
  version: 1
  train_id: <train>
  issue: <number>
  claim_nonce: <claim>
  task_id: <task>
  source_branch: repair/issue-<number>
  source_head: <sha>
  coordination_key: <key>
  integration_owner: <claim/session>
  accepted_at: <timestamp>
  paths_verified_non_overlapping: true
  coordination_key_verified_distinct: true
  acceptance_state: accepted
```

The accepted `source_head` is immutable for that train generation. Silent source drift fails closed. A successor head requires a new handoff, exact re-import, delivery-map update and affected validation rerun.

Import the exact accepted commit or a recorded contiguous Issue-owned commit range. Manual copying or squashing that destroys provenance is forbidden unless an equivalent deterministic mapping remains independently verifiable.

### Rollback declaration

Before freeze, record:

```yaml
rollback:
  mode: whole_train | independently_reconstructable
  per_issue:
    - number: <issue>
      source_head: <sha>
      integrated_commits:
        - <sha>
      paths:
        - <path>
      revert_strategy: <bounded strategy>
```

If one Issue cannot be safely separated or reverted, move it to a dedicated PR.

### Freeze

The train freezes before final audit, applicable E2E and exact-head CI.

After freeze:

- do not add another Issue;
- repair only findings within frozen scope;
- route unrelated findings to separate Issues;
- any final-head change invalidates affected audit, E2E and CI evidence;
- any accepted-source-head change requires re-handoff, re-import and revalidation before refreeze.

## Delivery mapping

Every reused, dedicated or train PR must contain a durable block equivalent to:

```yaml
repair_delivery:
  version: 1
  mode: reused_existing | dedicated | repair_train
  integration_owner: <claim/session>
  train_id: <train or null>
  issues:
    - number: <issue>
      claim_nonce: <claim>
      task_id: <task>
      source_branch: <branch>
      source_head: <sha>
      source_head_state: proposed | accepted_immutable | superseded
      accepted_at: <timestamp or null>
      coordination_key: <key>
      owned_paths:
        - <path>
      integrated_commits:
        - <sha>
      acceptance_evidence:
        - <evidence>
      validation:
        - <check or run>
      rollback:
        mode: whole_train | independently_reconstructable | dedicated
        strategy: <bounded strategy>
  freeze_head: <sha or pending>
  independent_audit:
    whole_diff: PASS | PENDING | FAILED
    per_issue:
      - number: <issue>
        result: PASS | PENDING | FINDING
        evidence:
          - <evidence>
  e2e:
    result: PASS | NOT_APPLICABLE | PENDING | FAILED
    reason: <reason or null>
  final_ci_head: <sha or pending>
  archive_state:
    status: active | completed_on_merge | completed
    effective_when:
      pull_request: <number>
      exact_head: <sha>
      merged: true
```

Human prose may supplement but cannot replace the mapping.

## Independent audit separation

The required final audit is valid only when the auditor:

- is a distinct agent/session/claim from the implementation owner;
- for a train, is distinct from every Issue worker and the integration owner;
- did not author, push, cherry-pick, rebase, amend, merge or remediate target commits;
- operates in `AUDIT ONLY` mode and does not mutate the target branch;
- audits the exact PR number, base SHA, head SHA and resulting diff;
- records one whole-diff verdict and one verdict for every included Issue.

`PASS` requires zero open material findings, `whole_diff: PASS`, and every per-Issue result `PASS`. Missing identity, independence attestation, base, head or per-Issue verdict leaves the audit `PENDING`.

Before role rotation, publish:

```yaml
audit_handoff:
  version: 1
  audit_generation: <integer>
  target:
    repository: blakinio/Oteryn-Platform
    pull_request: <number>
    base_sha: <sha>
    head_sha: <sha>
  implementation:
    owner: <claim/session>
    integration_owner: <claim/session or null>
    issue_workers:
      - issue: <number>
        claim_nonce: <claim>
        task_id: <task>
        source_head: <sha>
  scope:
    changed_paths:
      - <path>
    required_falsification_cases:
      - <case>
  evidence:
    delivery_mapping: <location>
    validation_runs:
      - <run or artifact>
    unresolved_threads: 0
  auditor_requirements:
    mode: AUDIT_ONLY
    independent_from_all_implementers: true
    exact_target_required: true
    whole_diff_and_per_issue_verdicts_required: true
```

Record the result:

```yaml
independent_audit:
  version: 1
  generation: <integer>
  auditor:
    actor: <actor>
    session_or_claim: <session-or-claim>
    mode: AUDIT_ONLY
  independence:
    distinct_from_implementation_owner: true
    distinct_from_integration_owner: true
    distinct_from_all_issue_workers: true
    wrote_target_commits: false
    mutated_target_branch: false
  target:
    pull_request: <number>
    base_sha: <sha>
    head_sha: <sha>
  whole_diff: PASS | FAILED | PENDING
  per_issue:
    - number: <issue>
      result: PASS | FINDING | PENDING
      evidence:
        - <evidence>
  material_findings_open: <integer>
  invalidated_by_target_change: false
```

Any effective target change invalidates that generation. The active auditor must not repair the target; findings return to the implementation owner. An auditor who writes a fix becomes an implementer and cannot issue final PASS for that generation.

A PASS-only audit is a review/comment on the delivery PR plus any linked audit record. It creates no audit PR.

## Parallel worker and audit-role routing

A command requesting `N` repair agents means `N` implementation workers unless it explicitly requests total slots. The coordinator must also ensure a separate audit-drain role is available when ready handoffs exist.

Recommended slot allocation:

```yaml
parallel_slots:
  2: {repair_workers: 1, audit_workers: 1}
  3: {repair_workers: 2, audit_workers: 1}
  4: {repair_workers: 2, audit_workers: 1, integration_coordinator: 1}
  5: {repair_workers: 3, audit_workers: 1, integration_coordinator: 1}
```

Workers that lose a deterministic lock immediately release and select another eligible Issue when authorized. Workers do not wait for peers, auditors or train capacity.

When a coherent delivery candidate reaches audit:

1. persist the exact handoff;
2. set the task checkpoint to `ready` with one audit `next_action`;
3. return `ROTATE` when a distinct eligible auditor cannot run in that session;
4. keep the Issue claim and branch durable for remediation/recovery;
5. use `WAITING` only for a genuine external dependency, accepted external review, permission/environment, observation window, protected operation, owner decision or exhausted terminal-CI limit.

The dedicated audit invocation drains the oldest valid ready audit handoff, remains read-only, and returns PASS or exact findings. A finding creates a new implementation/audit generation on the same delivery PR unless a separate root cause is proven.

## Lifecycle closeout relationship

These mechanisms are distinct:

- **repair train**: compatible active implementations delivered together;
- **audit artifact**: review/comment on the exact delivery PR;
- **lifecycle batch**: terminal governance-only reconciliation after implementation work is terminal.

Task archival included before merge must use `completed_on_merge` and may become `completed` only when the named PR exact head merges. Closing without merge must leave the task active, ready, waiting or blocked and must not release ownership.

## Coordinator safeguards

Dispatch multiple workers only when every selected Issue is ready, authorized, dependency-resolved, parallel-safe, unclaimed and non-overlapping. Coordinator intent does not acquire a lock or accept a train source head.

Do not hold completed work open to improve PR-count metrics. Do not absorb another owner's branch, task or PR. Shared paths remain serialized unless one explicit integration owner holds the lease.

## Metrics

```yaml
repair_economy_targets:
  duplicate_implementation_prs: 0
  audit_only_prs_per_repair: 0
  archive_only_prs_per_repair: 0
  unintentionally_open_related_prs_at_completion: 0
  workers_implementing_same_issue: 0
  trains_reverted_for_incoherent_scope: 0
  repair_prs_per_completed_issue: normally_lte_1
```

A lower PR count never overrides safety, review quality, acceptance, rollback, exact-head validation or complete closeout.
