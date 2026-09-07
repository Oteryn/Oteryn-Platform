# Oteryn Ecosystem Repository Migration Programme

```yaml
prompt_contract:
  version: 2.0.0
  programme_id: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
  objective: Prepare and execute one evidence-backed repository migration transaction without losing history, consumers, provenance, rollback or service safety.
  baseline_version: 1.1.0
  eval_suite: docs/agents/evals/prompt-contract-v1.json
  rollback_version: 1.1.0
  required_invariants:
    - tier2_requires_ready_transaction
    - mutation_verification_or_rollback
    - bounded_search_not_global_absence
    - protected_runtime_requires_separate_authority
  changed_surfaces:
    - transaction authority
    - cutover evidence
    - rollback and replay safety
owner_alias: OTERYN-REPO-MIGRATION
```

This reusable prompt is a migration task delta under the bound META organization policy and each repository’s trusted local bootstrap. Use `docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md` as durable state and resolve all coordinates live.

## Outcome and scope

Advance one bounded migration wave across only the repositories explicitly authorized for the task. A Platform task alone grants writes only in `Oteryn/Oteryn-Platform`; every other repository mutation needs its own exact authority and local governance.

Migration can include source extraction, destination preparation, repository creation/rename/transfer, workflow and package coordinate updates, compatibility bridges and archival. It does not authorize production deployment, protected runtime mutation, secrets, credentials, DNS/Cloudflare/Synology, live payments, live auth/game data or owner-funded external services.

Preserve product, service, auth, data, persistence, payment and deployment safety throughout preparation and cutover. Do not weaken branch rules, required reviews, tests, provenance, rollback or compatibility to make a migration pass.

## Evidence inventory

Before mutation, record exact source/destination identities and heads, repository permissions, path ownership, branch/ruleset state, releases/tags, packages/containers, Pages/environments, secrets/variables names without values, workflows/reusable callers, submodules, CODEOWNERS, dependency manifests, deployment/config references, documentation links, external consumers and rollback feasibility.

Classify callers as `PROVEN_CALLER`, scoped `PROVEN_NON_CALLER` or `UNOBSERVABLE_EXTERNAL_CALLER_RISK`. A bounded search never proves global absence. Preserve `UNKNOWN` and `CONFLICT`; cached SHAs, old approvals and prior successful commands are not current authority.

Classify a wave:

- **Tier 0:** read-only evidence and manifests;
- **Tier 1:** authorized repository preparation through task/branch/PR;
- **Tier 2:** physical create/rename/transfer/history cutover under the transaction below;
- **Tier 3:** protected runtime or live service mutation, requiring separate exact authority.

## Migration transaction

A Tier-2 action uses one durable `migration_transaction` with:

- stable transaction ID and one current owner;
- exact authorized source/destination repositories and protected heads;
- operation type and ordered mutation plan;
- complete caller/package/workflow/provenance inventory with residual unknowns;
- preconditions and invalidation conditions;
- rollback command/operation, rollback authority and point of no return;
- replay guard and idempotency behavior;
- post-mutation checks for repository identity, branches/tags/history, packages, workflows, permissions, links and consumers;
- states `DRAFT`, `PREPARED`, `READY_TO_EXECUTE`, `MUTATED_UNVERIFIED`, `COMPLETED` or `ROLLBACK_REQUIRED`.

`READY_TO_EXECUTE` means every required precondition and authority gate is current and the authorized runtime can perform the physical action. `CUTOVER_READY` is only a public programme status when exactly one named owner-only or unsupported physical operation remains; it is not permission or a transaction step.

Immediately before Tier 2, refresh permissions, protected heads, ownership, open conflicting work, rulesets, callers and rollback. Immediately after mutation, enter `MUTATED_UNVERIFIED` and run the recorded checks. Verification failure enters `ROLLBACK_REQUIRED`; do not continue downstream waves on narrative confidence.

## Migration correctness

For path extraction, prove inclusion/exclusion boundaries, history/tag behavior, license/security files, dependency cycles and build/test parity. For coordinate changes, update executable checkout/caller/workflow/package/release references and verify both producer and consumers. Documentation-only string replacement does not prove migration.

For package/container changes, preserve immutable provenance, permissions, visibility, signatures/attestations and rollback coordinates. Inaccessible package APIs remain residual unknowns.

Use compatibility bridges only when bounded, versioned, observable and removable. Define supported old/new coordinates, failure behavior, expiry/removal criteria and rollback. Never fabricate compatibility from a redirect that consumers do not follow.

## Acceptance delta

- Source/destination identity, ownership, history and permissions are verified at exact heads.
- Hidden callers, workflows, packages and external-consumer risk are explicitly classified.
- The transaction, replay guard, invalidation rules, rollback and post-mutation verification are complete.
- Required tests and CI prove extracted or relocated code and its consumers at the correct layer.
- Security boundaries, secrets handling, release provenance and protected rules remain intact.
- No production/runtime claim exceeds separately verified authority and evidence.
- Task, Issue, PR, transaction and programme state agree and record one next action.

A preparation/documentation-only slice records runtime E2E as `NOT_APPLICABLE` because no executable behavior changed. A physical migration requires direct resulting-state verification and any contract/runtime E2E named by the transaction. Persist exact coordinates, SHAs, transaction state, residual unknowns, rollback status and one next action in the canonical terminal response.
