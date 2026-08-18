---
task_id: OTERYN-20260818-remove-liquid20-out-of-scope
status: completed
repository: blakinio/Oteryn-Platform
implementation_authorized: true
implementation_pr: 1156
implementation_final_head: 4b3559c95fd7bfe70fd0a0278b187904820f67c1
implementation_merge: 88e1661bbd13ddb36b064d411d54702075f64852
completed_at: 2026-08-18T15:10:00Z
owner_direction: "Liquid20 and Freqtrade operational/control-plane material must not be part of Oteryn Platform."
---

# Terminal result — remove Liquid20 from Oteryn Platform scope

## Result

The active scope violation is removed from Oteryn Platform.

- `.github/workflows/liquid20-synology-control.yml` is removed;
- `deploy/liquid20/**` operational/control-plane assets are removed;
- `docs/agents/CI_WORKFLOW_LIFECYCLE.json` no longer registers that workflow and the workflow budget is `52`;
- `tests/ci/test_synology_deploy_release_identity.py` no longer treats Liquid20 as a Platform transfer/package component and includes a regression assertion that the out-of-scope operational paths are absent;
- current Platform transfer readiness and machine-readable transfer inventory no longer classify Liquid20 as Platform-owned;
- the generic Platform scope rule excluding Freqtrade remains authoritative and is strengthened in the closeout to name Liquid20/Freqtrade-derived operational workloads explicitly;
- no Freqtrade repository mutation, package mutation, self-hosted runner registration, Synology runtime mutation, staging operation or production operation was performed.

Historical commits, closed Issue #148, old PRs and archived evidence remain truthful provenance of the former scope error. They are not current Platform ownership or authority and must not be used to reintroduce Liquid20 operational control into this repository.

## Exact-head validation

Implementation final head `4b3559c95fd7bfe70fd0a0278b187904820f67c1`:

- CI `32152417125` — PASS;
- required `classify-changes` — PASS;
- full runtime tests — PASS;
- required aggregate `test` — PASS;
- Agent Governance `32152417059` — PASS;
- Build Synology Staging Images `32152417055` — PASS;
- Synology Production Target Preflight `32152417060` — PASS;
- Phase 7 Production-Like Validation `32152417064` — PASS;
- Platform DB Outage Validation `32152417044` — PASS;
- Edge Security Emulation `32152417045` — PASS;
- Native protocol contract `32152417042` — PASS;
- Native protocol contract audits `32152417081` — PASS;
- Game Auth Ticket Concurrency `32152417046` — PASS;
- workflow lifecycle — PASS (`52` actual / `52` budget);
- focused Platform transfer/scope tests — PASS (`15/15`);
- reviews `0`, inline review threads `0`, comments `0`.

## First failure and repair

The first PR head failed only governance metadata because the new active task had no `## Context checkpoint` and therefore no live PR/branch identity. Workflow lifecycle validation and focused scope-removal tests were already green on that head. The checkpoint was repaired, after which exact-head governance and CI passed without restoring any Liquid20 asset.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1156 squash-merged with repository auto-delete behavior and GitHub removed the implementation branch
source_branch_evidence: branch fix/remove-liquid20-from-platform-scope absent after merge 88e1661bbd13ddb36b064d411d54702075f64852
```

## Durable boundary

Liquid20/Freqtrade-derived collector, monitoring, deployment, package publication and operational control are **not Oteryn Platform product scope**. Any future work for them requires their own separately authorized repository/programme and must not be routed through Platform merely because a shared Synology host, GitHub runner or historical workflow once existed here.
