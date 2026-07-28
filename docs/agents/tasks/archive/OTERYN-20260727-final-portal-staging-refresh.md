---
task_id: OTERYN-20260727-final-portal-staging-refresh
required_reads:
  - AGENTS.md
  - .github/workflows/deploy-synology-staging.yml
  - docs/agents/ACTIVE_WORK.md
---

# OTERYN-20260727-final-portal-staging-refresh

## Goal

Build, deploy and verify the exact trusted-main portal release after PR #260 on the existing Synology staging target.

## Result

- PR #260 merged the final delivered-surface portal acceptance closure as `436d30e56bbf2821d01372a8aec15ec1a3ffca30`.
- PR #262 introduced the bounded one-shot staging refresh and merged as `ccd45fdce3176bd1da97a264bbbaf19a68c1397b`.
- PR #264 corrected runtime verification to execute inside the Platform container namespace and merged as `ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b`.
- PR #267 retriggered exact image publication and merged as `583cae5f430998b2bbdf5e60b59d93f09ec6f4c8`.
- One-shot run `30335134588` dispatched guarded Synology staging deployment run `30335161092` and completed with `PASS`.
- The deployed Platform and Gateway image references both exactly matched release tag `sha-583cae5f430998b2bbdf5e60b59d93f09ec6f4c8`.
- Health, homepage and protected Account surface verification succeeded inside the Platform container namespace.
- This is staging evidence only. Issue #91 remains the separate production go-live gate.

## Final checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T19:45:00Z
head: 95923546e6faa022bc77823768ad99de99f858c2
branch: ops/OTERYN-20260728-final-portal-image-trigger
pr: 267
merge_sha: 583cae5f430998b2bbdf5e60b59d93f09ec6f4c8
status: completed
context_routes:
  - agent-governance
  - testing
  - ci-repair
  - deployment
proven:
  - Every exact-head PR #267 workflow completed successfully before merge.
  - Exact Platform and Gateway images were published for trusted-main SHA 583cae5f430998b2bbdf5e60b59d93f09ec6f4c8.
  - Guarded Synology staging deployment run 30335161092 completed successfully.
  - One-shot run 30335134588 verified exact running image references and the refreshed portal runtime inside the Platform container namespace.
  - Issue #261 contains the sanitized PASS report for the exact deployed SHA.
derived:
  - The previously observed old portal was deployment drift, not an unresolved repository implementation gap.
unknown:
  - Real production behavior until Issue #91 is separately authorized and executed against an exact release.
conflicts: []
first_failure:
  marker: one-shot run 30309275896 verify job 90121994870
  evidence: host-loopback verification used the runner namespace instead of the Platform container namespace
rejected_hypotheses:
  - The guarded Synology deployment mechanism was broken.
  - The exact Platform or Gateway image identity could not be proven.
  - The portal failed its established deployment health check.
validation:
  - command: PR #267 exact-head workflows
    result: PASS
    evidence: CI, image build, Phase 7, DB outage, Game Auth and Edge Security all succeeded
  - command: one-shot run 30335134588
    result: PASS
    evidence: exact image resolution, guarded dispatch and in-container runtime verification succeeded
  - command: guarded deployment run 30335161092
    result: PASS
    evidence: Synology staging deployed exact release tag sha-583cae5f430998b2bbdf5e60b59d93f09ec6f4c8
blockers: []
next_action: None. Temporary one-shot workflow and trigger are removed by the archival cleanup PR.
```
