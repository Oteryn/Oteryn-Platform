# OTERYN-20260807 — CommonMark 2.9 security repair

## Task identity

- Mode: `REPAIR`
- Source issue: `#767`
- Delivery PR: `#768`
- Branch: `fix/issue-767-commonmark-2-9-security`
- Original base SHA: `842df4ac62bb6e928085f2bb328ff96259fa664e`
- Reconciled main SHA: `d581cc3b294de144e04a9ce373341b6d79af2269`
- Owner: `chatgpt-20260807-commonmark-security`
- Status: `READY_FOR_AUDIT`

## Problem statement

Final CI for PR #751 discovered six newly published Composer security advisories against the repository's locked `league/commonmark` 2.8.3. The failure is repository-wide and independent from #751. Upstream 2.9.0 is a dedicated security release addressing the affected ranges.

## Repair contract

1. Keep PR #751 unchanged and repair the repository dependency separately under Issue #767.
2. Update the lockfile from `league/commonmark` 2.8.3 to official 2.9.0 without ignoring advisories.
3. Do not change application/runtime behavior beyond the upstream dependency security fixes.
4. Validate the generated lock with Composer, security audit and exact-head repository CI.
5. Remove the temporary lock-generation workflow before candidate freeze.
6. Require an independent audit because this task originates from a CI security finding and is labelled `risk:high`.

## Evidence before mutation

- CI run `31117628046`, runtime job `92726946703`, failed at `composer audit --no-interaction`.
- Previous lock: `league/commonmark` 2.8.3 at source ref `1902f60f984235023acbe03db6ad614a37b3c3e7`.
- Upstream tag 2.9.0: `5703d83ba3da3b2e356a5fedc848ed6d8ffb6529`.
- Upstream changelog identifies 2.9.0 as a security release for five denial-of-service vulnerabilities and one unsafe-link/XSS class vulnerability.
- Root `composer.json` already permits 2.9.0 through `league/commonmark: ^2.8`; no root constraint change is necessary.

## Implementation evidence

- Temporary workflow run `31154182915` completed successfully.
- Composer generated the lock with `composer update league/commonmark --no-interaction --no-progress`.
- The same job passed `composer validate --strict` and `composer audit --no-interaction`.
- Generated lock pins `league/commonmark` 2.9.0 to upstream source/dist ref `5703d83ba3da3b2e356a5fedc848ed6d8ffb6529`.
- The temporary workflow was deleted before candidate freeze and is absent from the effective diff.
- The branch was reconciled with `main@d581cc3b294de144e04a9ce373341b6d79af2269` using a merge tree that preserved the current-main lifecycle closeout and the two security-repair paths.
- Effective diff against current main is exactly `composer.lock` plus this task record; `behind_by=0` before this task-state commit.

## Audit-risk decision

```yaml
audit_gate:
  version: 1
  requirement: REQUIRED
  classified_by: chatgpt-20260807-commonmark-security
  classified_at: 2026-08-07T06:28:33Z
  risk: high
  mandatory_triggers:
    - risk:high source Issue
    - CI security finding requires remediation
  optional_triggers: []
  disproved_triggers:
    - no authentication authorization session credential or payment boundary is intentionally changed
    - no database schema migration deployment public protocol or CI policy change remains in the effective diff
  unknown_or_conflict: []
  rationale: The final effective change is narrow and reversible but directly remediates high-severity dependency advisories discovered by the required CI security gate; repository policy therefore requires a distinct independent auditor.
  self_review:
    result: PASS
    evidence:
      - Composer generated the lock rather than a hand-written lockfile edit.
      - composer validate and composer audit passed in generator run 31154182915.
      - generated package version and source reference exactly match official upstream 2.9.0.
      - final effective diff contains only composer.lock and this task record.
      - temporary workflow is absent from the final effective diff.
      - rollback is a bounded revert of the dependency-update PR.
  independent_audit:
    result: PENDING
    generation: 1
    evidence:
      - exact immutable audit handoff is published after this candidate-state commit and must be performed by a distinct auditor.
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: recorded-in-pr-768-freeze-comment
  acceptance_checked: true
  full_diff_checked: true
  related_prs_checked: true
  negative_paths_checked: true
  rollback_checked: true
  findings:
    - none
  evidence:
    - Composer-generated diff changes CommonMark metadata from 2.8.3 to 2.9.0 and official upstream refs.
    - Root composer.json is unchanged because ^2.8 already permits 2.9.0.
    - No advisory-ignore configuration was introduced.
    - No application source, schema, deployment, public contract, auth or CI workflow remains changed in the effective candidate.
    - PR 751 remains a separate bounded Wiki fixture repair and Issue 365 remains outside this security task.
```

## Validation plan

- generator evidence: `composer validate --strict` PASS
- generator evidence: `composer audit --no-interaction` PASS
- final effective-diff check: exactly two owned paths
- required exact-head repository workflows must be terminal success
- distinct independent audit must return PASS on the unchanged frozen target

## Rollback

Revert the bounded dependency-update PR to restore the previous lockfile. Do not suppress audit findings as rollback.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T06:34:00Z
head: derive-from-live-pr-768
branch: fix/issue-767-commonmark-2-9-security
pr: 768
issue: 767
status: ready
context_routes:
  - agent-governance
  - security
  - dependencies
  - wiki
owned_paths:
  - composer.lock
  - docs/agents/tasks/active/OTERYN-20260807-commonmark-2-9-security.md
proven:
  - Required CI for PR 751 exposed six security advisories in locked league/commonmark 2.8.3.
  - Upstream league/commonmark 2.9.0 is an official security release outside the reported affected ranges.
  - Root composer.json already admits 2.9.0 under ^2.8.
  - Temporary generator run 31154182915 used Composer to generate the lock and passed composer validate plus composer audit.
  - Generated composer.lock pins league/commonmark 2.9.0 to official source ref 5703d83ba3da3b2e356a5fedc848ed6d8ffb6529.
  - The temporary generation workflow is deleted and absent from the effective diff.
  - Reconciliation with current main retained an effective diff of exactly composer.lock and this task record.
  - Implementation self-review is PASS.
derived:
  - The new dependency security failure was not caused by PR 751 but must be repaired on main before PR 751 can obtain a clean current security gate.
  - Independent audit is mandatory because the repair is high-risk/security-finding driven even though the effective diff is small.
unknown:
  - Terminal conclusion of all workflows emitted for the frozen final head.
  - Independent auditor verdict on the frozen final head.
conflicts: []
first_failure:
  marker: composer-security-audit-commonmark-2.8.3
  evidence: CI run 31117628046 runtime job 92726946703 failed composer audit after successful dependency installation.
rejected_hypotheses:
  - Ignore or suppress the new Composer advisories.
  - Add the dependency repair to PR 751 and contaminate its Issue scope.
  - Hand-edit composer.lock without Composer generation evidence.
changed_paths:
  - composer.lock
  - docs/agents/tasks/active/OTERYN-20260807-commonmark-2-9-security.md
validation:
  - command: Temporary CommonMark Lock Generator run 31154182915 / composer update league/commonmark
    result: PASS
    evidence: Composer generated a lock containing league/commonmark 2.9.0.
  - command: Temporary CommonMark Lock Generator run 31154182915 / composer validate --strict
    result: PASS
    evidence: Generated lock and manifest validated successfully.
  - command: Temporary CommonMark Lock Generator run 31154182915 / composer audit --no-interaction
    result: PASS
    evidence: Security audit completed successfully after the 2.9.0 update.
blockers:
  - Exact-head repository CI must complete successfully.
  - A distinct independent auditor must return PASS for generation 1.
next_action: Publish the exact immutable generation-1 audit handoff for PR 768, verify final exact-head CI, then merge only after independent PASS with an unchanged target.
```
