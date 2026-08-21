---
task_id: OTERYN-20260821-atlas-creature-preview-deploy
status: completed
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
phase: closeout
issue: 1191
cross_repository_issue: Oteryn/Oteryn-Atlas#30
---

# Atlas creature preview deployment — terminal closeout

## Result

The Atlas static-creature product and its desktop/mobile global-search acceptance are now owned and executed by `Oteryn/Oteryn-Atlas` on the dedicated organization runner route. Platform no longer needs the temporary Atlas publication/E2E implementation that was carried in `repair-synology-autostart.yml`.

## Terminal evidence

- Atlas replacement-route PRs: `Oteryn/Oteryn-Atlas#44`, `#45`, `#46`.
- Exact successful Atlas main head: `1e0f021fc7a723de807e86d53a26dd0564a5ef23`.
- Trusted-main run/job: `32526864123` / `96911114022`, conclusion `success`.
- Runner contract: `atlas-runners` / `oteryn-atlas` / `oteryn-synology-atlas`, organization scope; Docker capability present; Platform staging-state absent.
- Exact product: 88,633 placements, 5,746 chunks, 1,945 search records; semantic digest `sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1`.
- Live publication, exact served bytes, seven retained root contracts and HTTP Range verification: PASS.
- Real Chromium: desktop PASS and mobile PASS.
- Browser artifact `9463015639`, `atlas-synology-live-e2e-32526864123-1`, digest `sha256:9582ac4fa7b388498aab22f64911f66e53ed3c5059ff0eef505065ba8beeece0`, retained through 2026-09-20.
- Artifact result: desktop selected `npc:473280040fcebbf2bc1bad9e3717d7a9`; mobile selected `monster:83f049b79b2988bccdfb22f9a46a739d`.
- Task-owned Atlas execution resources cleaned successfully after the passing run; no rollback executed after browser PASS.

## Platform cleanup in this closeout

- `.github/workflows/repair-synology-autostart.yml` restored exactly to trusted blob `f3959e6bea09d39920db0e5515770a1ec77114ca`.
- `deploy/synology/compose.yml` restored exactly to trusted blob `fd9d3bb5e8288be091480b2860a870d662a196df`.
- Temporary `scripts/acceptance/atlas-creature-preview-e2e.cjs` removed.
- Temporary `deploy/ci/playwright-chromium.Dockerfile` removed.
- Atlas host-local ownership now resides in `Oteryn/Oteryn-Atlas`; the Platform repair workflow returns to Platform-only restart-policy enforcement.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: terminal Platform cleanup PR #1212 releases the completed Atlas preview execution ownership
source_branch_evidence: Atlas replacement route passed run 32526864123 and repository auto-delete applies after the exact-head squash merge
```

## Self-review

Full cleanup diff is intentionally bounded to the temporary Atlas execution scaffold plus these task archival records. No product runtime, database, authentication, payment, secret, or production configuration is changed by the Platform cleanup candidate.
