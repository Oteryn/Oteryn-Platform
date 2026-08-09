---
task_id: OTERYN-20260809-download-artifact-immutability-audit
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-content
task_kind: audit
implementation_authorized: false
execution_mode: github
execution_reason: Download Center security audit was fully evidenced in the canonical WWW Platform repository
status: completed
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
search_first:
  - Issue #948
  - PR #161
  - historical Download Center closeout Issues #562/#622/#647/#656/#676/#679/#682
optional_reads: []
---

# OTERYN-20260809-download-artifact-immutability-audit

## Goal

Audit the delivered Download Center artifact-reference boundary against the accepted requirement that public client releases reference immutable, operator-approved artifacts rather than merely well-formed URLs on a trusted host.

## Terminal result

One material finding was proven and routed without remediation in the audit role:

- **OPA-SEC-0008 / Issue #948 — HIGH / P1**: accepted Download Center architecture requires immutable artifact references, but the delivered `ArtifactUrlPolicy` accepts any HTTPS URL on an exact allowlisted host when it has any non-root path. An approved mutable alias or overwriteable object key can therefore pass publication without content-address, object-version, digest-binding or equivalent immutability proof.

The public checksum notice remains truthful: SHA-256 is administrator-supplied and Platform explicitly does not fetch or independently verify the artifact. That deliberate boundary was not reported as the defect.

Issue #948 remains the independent remediation owner according to live closeout-time state.

## Acceptance criteria

- [x] Refreshed protected main, active tasks, open PRs and independent audit-repair owners.
- [x] Preserved non-overlapping owners and did not enter Issue #948 remediation paths.
- [x] Audited the accepted Download Center architecture and delivered PR #161 implementation.
- [x] Falsified artifact-reference immutability with an approved-host mutable-reference negative path.
- [x] Distinguished the finding from truthful administrator-supplied checksum/no-fetch semantics.
- [x] Deduplicated and routed OPA-SEC-0008 / Issue #948.
- [x] Merged bounded audit PR #949.
- [x] Reconciled concurrent federated-search repair/lifecycle work without reverting or claiming it.
- [x] Repaired the material Codex P2 checkpoint/ancestry finding before merge.
- [x] Exact-final-head self-review passed.
- [x] Fresh Codex exact-head review reported no major issues.
- [x] Agent Governance and repository CI passed on the exact final audit head.
- [x] All review threads were resolved; zero unresolved material threads remained.
- [x] Runtime/browser E2E was NOT_APPLICABLE for this documentation-only audit.

## Terminal evidence

```yaml
checkpoint_version: 1
updated_at: 2026-08-09T12:07:00Z
head: 7bc6987f5869f19e82ebb7ee60ca621850fc530e
branch: audit/OTERYN-20260809-download-artifact-immutability
pr: 949
status: completed
phase: lifecycle-closeout
session_role: auditor
project_lane: oteryn-platform-content
execution_mode: github
context_routes:
  - agent-governance
  - security
  - public-web-cms
  - architecture
  - downloads
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability-audit.md
  - docs/agents/reports/OTERYN-20260809-download-artifact-immutability-audit.md
proven:
  - Protected main at audit selection was c1b1d26b355db26a89d983cc4abc6477bf843a26.
  - The accepted PUBLIC_WEBSITE_EXPANSION_PLAN requires immutable artifact URLs or approved immutable storage references for Download Center releases.
  - ArtifactUrlPolicy enforces HTTPS, exact configured host, no userinfo, no fragment, standard HTTPS port and a non-root path, but no content-address, object-version, digest binding or equivalent immutable-reference proof.
  - PublishClientRelease re-runs the same insufficient URL policy immediately before publication.
  - SaveClientReleaseRequest requires SHA-256 syntax but does not bind that administrator-supplied digest to the referenced bytes.
  - Existing focused tests did not reject mutable aliases or overwriteable paths on an otherwise approved host.
  - Duplicate searches found no Issue owning this exact artifact-reference immutability root cause.
  - OPA-SEC-0008 / Issue #948 was created as the independent remediation owner.
  - Concurrent PR #947 advanced main and its separate lifecycle closeout #950 merged as b87deb370c4a0a629a8aaf05d0447134f2ee823e; the audit did not take ownership of those paths.
  - Codex review on intermediate head a10c92a495eb879e78fca2d2e2c20843ffeb84dd found a P2 checkpoint defect: the branch had not actually incorporated #950 through its merge base.
  - The audit branch was rebuilt directly on main@b87deb370c4a0a629a8aaf05d0447134f2ee823e and the checkpoint claim was corrected before final validation.
  - Final audit PR #949 head was 7bc6987f5869f19e82ebb7ee60ca621850fc530e.
  - Agent Governance run 31307716207 passed on the exact final audit head.
  - CI run 31307716206 passed on the exact final audit head; classify-changes PASS, test PASS, runtime-tests SKIPPED.
  - Exact final diff contained only the audit task and audit report.
  - Exact-head self-review on 7bc6987f5869f19e82ebb7ee60ca621850fc530e passed with zero material findings.
  - Fresh Codex review explicitly reviewed 7bc6987f58 and reported no major issues.
  - The prior P2 review thread is resolved and outdated; zero unresolved material review threads remain.
  - PR #949 merged as 698ec482d326c3281377416419de37f6756273d8.
  - Runtime/browser E2E was NOT_APPLICABLE because the audit changed documentation only.
  - At closeout refresh OPA-SEC-0005 / Issue #938 is closed/completed after independent repair and lifecycle closeout.
  - At closeout refresh OPA-SEC-0006 / Issue #941, OPA-SEC-0007 / Issue #944 and OPA-SEC-0008 / Issue #948 are open, priority P1, risk high, agent:ready and unclaimed; future ownership remains live-query-derived.
  - PR #541 remains an open draft public-domain external-evidence package and PR #338 remains an open draft Game Catalog schema 1.3 compatibility hold.
derived:
  - Database-row immutability does not make externally addressed bytes immutable when the approved URL can resolve to replaced content.
  - Manual checksum comparison is useful but is not equivalent to the architecture's machine-enforced immutable-reference invariant.
  - The finding is a delivered Download Center policy defect, but the audit does not prove that a malicious artifact has actually been served or that the configured production host is mutable.
unknown:
  - Future claim/remediation state of Issues #941, #944 and #948 after this closeout generation.
  - Production artifact-host storage immutability properties outside repository evidence.
conflicts: []
first_failure:
  marker: exact-head-review-ancestry-evidence
  evidence: Codex P2 on intermediate head a10c92a correctly disproved the checkpoint statement that #950 was incorporated through the branch merge base; branch ancestry and checkpoint evidence were repaired before the final PASS generation.
rejected_hypotheses:
  - Exact host allowlisting proves artifact immutability; it constrains origin but not overwriteability.
  - A non-root or version-looking pathname proves immutable bytes; naming convention is not a storage immutability proof.
  - Lack of independent checksum verification is the defect; that limitation is explicitly disclosed and outside this root cause.
  - Historical Download Center lifecycle Issues already own the defect; they repaired ownership lifecycle and preserved supplied-checksum/no-fetch semantics.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability-audit.md
  - docs/agents/reports/OTERYN-20260809-download-artifact-immutability-audit.md
validation:
  - command: Agent Governance run 31307716207
    result: PASS
    evidence: exact final audit head 7bc6987f5869f19e82ebb7ee60ca621850fc530e
  - command: CI run 31307716206
    result: PASS
    evidence: classify-changes PASS; test PASS; runtime-tests SKIPPED
  - command: fresh Codex exact-head review
    result: PASS
    evidence: reviewed commit 7bc6987f58; no major issues
  - command: review threads
    result: PASS
    evidence: prior P2 thread resolved/outdated; zero unresolved material threads
  - command: merge verification
    result: PASS
    evidence: PR #949 merged as 698ec482d326c3281377416419de37f6756273d8
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: documentation-only audit; no product behavior changed
blockers: []
next_action: Refresh live findings, active tasks, deterministic repair branches and open PRs before selecting the next non-overlapping continuous-audit domain; treat Issues #941, #944 and #948 as exclusions only while fresh live state proves them active.
```

## Remediation boundary

Issue #948 exclusively owns the bounded Download Center artifact-reference immutability repair declared in that Issue. This archived audit does not authorize or perform that runtime/config/test repair, executable upload/proxy/fetch behavior, code signing, storage provisioning, deployment, production mutation or external-repository work.
