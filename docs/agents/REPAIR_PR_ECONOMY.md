# Repair PR and Actions Economy

```yaml
repair_pr_economy_version: 3
repository: blakinio/Oteryn-Platform
default_delivery: one_issue_one_owner_one_pr
mandatory_external_repair_audit: false
validation_gate: docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
```

## Default delivery

Ordinary remediation uses:

```text
one Issue
→ one implementation owner
→ one deterministic branch
→ one authoritative delivery PR
→ exact-head self-review and validation
→ merge
→ Issue closure, archival and ownership release by the same owner
```

The PR is a delivery and review artifact, not an ownership primitive. Issue ownership remains with the implementation owner until terminal closeout.

Do not create activity-only PRs, audit PRs, audit Issues, integration PRs or repair trains for an ordinary coherent repair.

## Commit and push economy

- Build a coherent set of edits before pushing.
- Prefer one reviewable implementation commit or a small number of meaningful commits.
- Never create one commit per file, checkpoint field, comment, review response or evidence line merely because the GitHub contents API is used.
- Update durable task state at material boundaries, not after every tool call.
- Run cheap focused checks during construction and one full applicable exact-head validation at final readiness.
- Avoid no-op commits and unnecessary merge-base refreshes after readiness.
- Use squash merge for the final delivery unless repository policy requires another method.

When a tool cannot create an atomic multi-file commit, stage coherent changes without triggering repeated validation where possible, then squash the final PR. Tool limitations do not justify dozens of Actions generations.

## Actions routing

Documentation, task, evidence and agent-governance-only diffs use lightweight CI/governance checks. Edge, outage, production-like and concurrency workflows start only when their runtime, dependency, contract, deployment, workflow or routing surfaces are affected.

Supersedable PR workflows use a concurrency key scoped to workflow plus PR/ref and:

```yaml
cancel-in-progress: true
```

A newer head invalidates older same-workflow results and should automatically cancel their queued or running executions.

## Checkpoint rule

A checkpoint-only, task-record-only or agent-governance-only commit must not launch unrelated heavy runtime workflows. Such a commit may run the lightweight required CI and Agent Governance checks selected by path policy.

Checkpoint updates are batched with the next coherent implementation change whenever that does not reduce recovery safety. Do not publish a new checkpoint head merely to restate unchanged progress.

## Runtime validation rule

Full runtime, edge, outage, production-like and concurrency validation runs only when corresponding path classification or workflow changes require it. Classification failure remains fail-closed.

A skipped heavy job is not proof of runtime behavior; it is proof only that the changed paths did not require that gate.

## Findings

Findings from self-review, CI or ordinary PR review return to the same implementation owner. The owner repairs the branch, reruns focused checks and final exact-head gates, then completes closeout.

No different-agent PASS is required for repair merge.

## Exceptional batching

Lifecycle-only reconciliation may be batched when changes are homogeneous, reversible and preserve exact per-Issue provenance. Ordinary product/runtime repairs remain one Issue and one PR and never wait to fill a train.

## Closeout gate

Merge requires:

- acceptance criteria satisfied;
- exact-head full-diff self-review `PASS`;
- risk-proportional focused tests;
- applicable E2E complete;
- repository-required exact-head CI `PASS`;
- bounded rollback and compatibility evidence where applicable;
- zero material findings, requested changes, unresolved review threads, blockers or ownership conflicts;
- intentional terminal states for related PRs.

After merge, verify resulting state, close the Issue, archive the task and release ownership.
