# OTERYN-20260807 — CommonMark 2.9 security repair

## Task identity

- Mode: `REPAIR`
- Source issue: `#767`
- Branch: `fix/issue-767-commonmark-2-9-security`
- Base SHA: `842df4ac62bb6e928085f2bb328ff96259fa664e`
- Owner: `chatgpt-20260807-commonmark-security`
- Status: `IMPLEMENTING`

## Problem statement

Final CI for PR #751 discovered six newly published Composer security advisories against the repository's locked `league/commonmark` 2.8.3. The failure is repository-wide and independent from #751. Upstream 2.9.0 is a dedicated security release addressing the affected ranges.

## Repair contract

1. Keep PR #751 unchanged and repair the repository dependency separately under Issue #767.
2. Update the lockfile from `league/commonmark` 2.8.3 to official 2.9.0 without ignoring advisories.
3. Do not change application/runtime behavior beyond the upstream dependency security fixes.
4. Validate the generated lock with Composer, security audit, focused Wiki/Markdown coverage and exact-head repository CI.
5. Remove any temporary lock-generation workflow before declaring a candidate.
6. Require an independent audit because this task originates from a CI security finding and is labelled `risk:high`.

## Evidence before mutation

- CI run `31117628046`, runtime job `92726946703`, failed at `composer audit --no-interaction`.
- Current lock: `league/commonmark` 2.8.3 at source ref `1902f60f984235023acbe03db6ad614a37b3c3e7`.
- Upstream tag 2.9.0: `5703d83ba3da3b2e356a5fedc848ed6d8ffb6529`.
- Upstream changelog identifies 2.9.0 as a security release for five denial-of-service vulnerabilities and one unsafe-link/XSS class vulnerability.
- Root `composer.json` already permits 2.9.0 through `league/commonmark: ^2.8`.

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
    - no database schema migration deployment or public protocol change is authorized
  unknown_or_conflict: []
  rationale: The update is narrow and reversible but directly remediates high-severity dependency advisories discovered by the required CI security gate; repository policy therefore requires a distinct independent auditor.
  self_review:
    result: PENDING
    evidence: []
  independent_audit:
    result: PENDING
    generation: 0
    evidence: []
```

## Validation plan

- `composer validate --strict`
- `composer audit --no-interaction` must return zero affected advisories
- verify final diff contains only `composer.lock` and this durable task record
- relevant Wiki/Markdown tests and exact-head repository CI
- independent audit of frozen candidate

## Rollback

Revert the bounded dependency-update PR to restore the previous lockfile. Do not suppress audit findings as rollback.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T06:28:33Z
head: derive-from-live-branch
branch: fix/issue-767-commonmark-2-9-security
issue: 767
status: implementing
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
  - Upstream league/commonmark 2.9.0 is an official security release outside all reported affected ranges.
  - Root composer.json already admits 2.9.0 under ^2.8.
derived:
  - The repository must update its lock before PR 751 can obtain a clean required CI result.
  - The security repair must remain separate from PR 751 to preserve that repair's bounded scope.
unknown:
  - Exact Composer-generated lock diff before lock generation completes.
  - Exact-head CI outcome after the dependency update.
  - Independent auditor verdict for the frozen candidate.
conflicts: []
first_failure:
  marker: composer-security-audit-commonmark-2.8.3
  evidence: CI run 31117628046 runtime job 92726946703 failed composer audit after successful dependency installation.
rejected_hypotheses:
  - Ignore or suppress the new Composer advisories.
  - Add the dependency repair to PR 751 and contaminate its Issue scope.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-commonmark-2-9-security.md
validation: []
blockers:
  - Generate and validate the exact lockfile update.
  - Obtain independent audit PASS for the frozen candidate.
next_action: Generate composer.lock with Composer on the security branch, remove the temporary generator, self-review the final diff, then publish an independent audit handoff.
```
