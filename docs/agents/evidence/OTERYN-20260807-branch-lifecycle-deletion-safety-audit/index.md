# OTERYN-20260807 branch lifecycle deletion-safety audit evidence

- Repository: `blakinio/Oteryn-Platform`
- Audited main: `021bf44d99de4430b2e054d25872eabfa322eba2`
- Domain: destructive branch lifecycle apply path
- Accepted authority: `docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md`
- Policy: `docs/agents/BRANCH_LIFECYCLE_POLICY.json`
- Workflow: `.github/workflows/branch-lifecycle.yml`
- Implementation: `tools/agents/branch_lifecycle.py`
- Original implementation: Issue #658 / PR #666; terminal and ownership released
- Confirmed finding: `OPA-GOV-0019`
- Remediation Issue: #780
- Severity: high
- Confidence: high
- Evidence state: PROVEN

## Exact proof chain

1. `fetch_live_snapshot()` resolves mutable branch/PR/task/Issue evidence once.
2. `classify_snapshot()` determines `TERMINAL_MERGED` from that snapshot.
3. `validate_manifest()` compares reviewed entries against the same in-memory report.
4. `apply_manifest()` iterates validated entries and invokes `delete_branch(branch)` by ref name.
5. There is no per-entry live re-fetch of ref SHA, PR, task/claim or protection state immediately before deletion.
6. Branch Lifecycle workflow concurrency does not prevent ordinary candidate-branch writes or ownership activation.

This proves a time-of-check/time-of-use window in a destructive Git-ref operation.

## Safety boundary

No production, staging, external repository, repository setting or candidate branch was mutated. No destructive race reproduction was attempted.
