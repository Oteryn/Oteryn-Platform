# CI change-routing evidence

Issue: #467  
Implementation PR: #468

## Baseline

Documentation/governance PR #453 triggered five unrelated runtime-heavy workflow families: CI, Phase 7 Production-Like Validation, Edge Security Emulation, Platform DB Outage Validation and Game Auth Ticket Concurrency. The baseline audit recorded this as a P0 cost and queue-pressure defect.

## Implemented boundary

- One repository-owned classifier maps changed paths to 13 declared change classes.
- Unknown paths, mixed risk, dependencies, database/migrations, auth/security, payments, deployment, contracts, workflow self-changes and deletions fail closed.
- Root Markdown and `docs/**` may be documentation-only, except `docs/contracts/**`, which is shared risk.
- `ops/**` remains deployment risk even when the file extension is Markdown.
- Pure Game Gateway changes remain handled by the separate path-scoped Game Gateway CI and do not start these five Platform-heavy internals.
- Each affected workflow still emits its original terminal job identity (`test`, `validate` or `concurrency-proof`). Workflow-level path filters were not added.
- A classifier failure causes the original required job to run and fail before heavy validation, rather than silently skipping.
- Every classifier invocation validates the deterministic fixture contract before routing. Four workflows also run the standalone unit suite explicitly; Phase 7 invokes the same self-validating classifier directly.
- Skipped jobs are routing evidence only and are not represented as product-validation evidence.

## Deterministic cases

The fixture ledger covers the 13 baseline classes plus fail-closed boundaries for:

- mixed documentation and dependency changes;
- unknown paths;
- classifier/workflow self-change;
- shared contracts;
- operational Markdown;
- unknown nested Markdown;
- file deletions.

## Remote generation evidence

- Bootstrap run `30746779996` passed classifier tests and exact generated-diff checks, then its ref update was rejected because the Actions token lacked workflow-write permission. No remote workflow mutation occurred.
- Artifact run `30746937197` proved the same generator but exposed hidden-directory exclusion by `upload-artifact`.
- Final generator run `30747105458` passed and produced artifact `8833224700` with digest `sha256:c769eccedeec558d66ca5c34c9e4fa9f4c0c3a740dc1e063374651fdd246abf0`.
- All five artifact SHA-256 entries verified locally before Git object creation.
- The previously rejected local commit `83070e76eed5fb657ad7649fa8527f7599013cdc` remained available as a dangling Git object and supplied the exact tested Phase 7 blob without copying or truncating the 29 KB workflow.

## Final implementation paths

Temporary bootstrap instrumentation and its patch script are deleted in the implementation commit. The durable implementation consists only of:

- five modified workflow definitions;
- `scripts/ci/classify_changes.py`;
- deterministic fixtures and tests;
- this evidence and the active task checkpoint.

Production, database content, protected secrets, external repositories and payment activation are unchanged.
