# Docs-only routing proof

## Purpose

This closeout pull request changes only repository documentation under `docs/agents/**`. It is the real-system acceptance probe for the change classifier merged in PR #468 as `6af891c47adfba0177372b54419a831b51fa6c09`.

## Expected observable behavior

- each affected workflow emits and passes its `classify-changes` job;
- the classifier reports `agent_governance` and no heavy gates;
- existing terminal jobs `test`, `validate`, and `concurrency-proof` are present but skipped by job-level routing;
- no Composer installation, MariaDB, Redis, MailHog, nginx, Laravel runtime, database outage, edge emulation, or game-auth concurrency internals start;
- Agent Governance passes;
- a skipped terminal job is routing evidence only and is not represented as product-validation evidence.

## Evidence status

`PENDING_EXACT_HEAD_VALIDATION`

The terminal run IDs, job conclusions, exact head, merge identity, and archival evidence will be recorded after GitHub Actions completes on this pull request. No production or runtime environment is modified by this probe.
