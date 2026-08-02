# CI change-routing evidence

Issue: #467  
Implementation PR: #468  
Closeout/proof PR: #469

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
- All five artifact SHA-256 entries verified before Git object creation.
- The previously rejected local commit `83070e76eed5fb657ad7649fa8527f7599013cdc` remained available as a dangling Git object and supplied the exact tested Phase 7 blob without copying or truncating the workflow.

## Implementation delivery

Implementation head `2bde126b6e918e9a9d6beb2b6fffa7c06f157790` passed all exact-head workflows:

- Agent Governance — `30747542044`;
- CI — `30747542053`;
- Phase 7 Production-Like Validation — `30747542042`;
- Edge Security Emulation — `30747542041`;
- Platform DB Outage Validation — `30747542045`;
- Game Auth Ticket Concurrency — `30747542043`.

PR #468 merged to `main` as `6af891c47adfba0177372b54419a831b51fa6c09`.

## Real docs-only routing proof

PR #469 head `f600f32a944a618cae10b6eefba5c743b6452e2e` changed exactly one path under `docs/agents/**`. All six workflow runs succeeded. In each of the five routed workflow families, `classify-changes` passed and the original terminal job concluded `skipped` with `steps: null`:

- CI run `30748126711`: classifier job `91497188125`; `test` job `91497217003` skipped;
- Phase 7 run `30748126716`: classifier job `91497188242`; `validate` job `91497205839` skipped;
- Edge run `30748126722`: classifier job `91497188229`; `validate` job `91497208032` skipped;
- DB Outage run `30748126727`: classifier job `91497188257`; `validate` job `91497207026` skipped;
- Game Auth run `30748126715`: classifier job `91497188186`; `concurrency-proof` job `91497202804` skipped;
- Agent Governance run `30748126720` passed.

This proves the workflows and stable terminal job identities remain emitted while Composer, MariaDB, Redis, MailHog, nginx, Laravel runtime, outage, edge-emulation and concurrency internals do not start for the proven-safe documentation class.

## Final durable paths

Temporary bootstrap instrumentation and its patch script are absent. The durable implementation consists only of:

- five modified workflow definitions;
- `scripts/ci/classify_changes.py`;
- deterministic fixtures and tests;
- this evidence set;
- the archived terminal task record.

Production, database content, protected secrets, external repositories and payment activation are unchanged.
