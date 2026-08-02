# Docs-only routing proof

## Purpose

This closeout pull request changes only repository documentation under `docs/agents/**`. It is the real-system acceptance probe for the change classifier merged in PR #468 as `6af891c47adfba0177372b54419a831b51fa6c09`.

## Exact probe

- pull request: #469
- head: `f600f32a944a618cae10b6eefba5c743b6452e2e`
- changed paths: exactly `docs/agents/evidence/OTERYN-20260802-ci-change-routing/docs-only-proof.md`
- expected class: `agent_governance`
- result: `PASS`

## Observable workflow evidence

| Workflow | Run | Classifier job | Original terminal job | Result |
|---|---:|---:|---:|---|
| CI | `30748126711` | `91497188125` — success | `91497217003` `test` — skipped | PASS |
| Phase 7 Production-Like Validation | `30748126716` | `91497188242` — success | `91497205839` `validate` — skipped | PASS |
| Edge Security Emulation | `30748126722` | `91497188229` — success | `91497208032` `validate` — skipped | PASS |
| Platform DB Outage Validation | `30748126727` | `91497188257` — success | `91497207026` `validate` — skipped | PASS |
| Game Auth Ticket Concurrency | `30748126715` | `91497188186` — success | `91497202804` `concurrency-proof` — skipped | PASS |
| Agent Governance | `30748126720` | applicable governance job — success | not applicable | PASS |

Each skipped original terminal job returned `steps: null`. Therefore no job setup, service container initialization, Composer installation, MariaDB, Redis, MailHog, nginx, Laravel runtime, database outage, edge emulation, or game-auth concurrency step started.

## Interpretation

- The workflow files remained triggered, so stable required workflow/check behavior was preserved.
- The repository-owned classifier executed and passed before routing.
- Existing terminal job identities remained present and concluded `skipped` only after successful classification.
- The skip is routing evidence only; it is not product-validation evidence.
- Unknown/shared/security/deployment/workflow changes remain fail-closed through the separately validated classifier fixture contract.

## Production state

`NOT_CHANGED`. No application runtime, production environment, database content, payment activation, protected secret, deployment target, or external repository was modified.
