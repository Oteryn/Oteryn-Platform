# Oteryn Platform agent instructions

These instructions govern `Oteryn/Oteryn-Platform`, repository ID `1305155726`.

## Authority

- `Oteryn/Oteryn-Platform` is the canonical Platform write authority. `blakinio/Oteryn-Platform` is a historical pre-transfer coordinate only.
- Platform owns portal/web identity, commercial/control-plane services, deployment/runtime infrastructure and Platform-owned contracts. Game and Atlas repositories are separate authorities and are read-only unless the current owner task explicitly grants writes there.
- Read a nearer `AGENTS.md` for touched paths. A same-directory `AGENTS.override.md` replaces, rather than extends, the base instruction file and therefore exists only when intentional replacement semantics are required.

## Lifecycle

- GitHub Issue is authoritative for substantial task status, dependencies and acceptance criteria.
- Use one independently mergeable task -> one branch -> one PR. Read-only research/review normally creates no branch.
- Markdown task packets may hold bounded execution detail but must not become a second mutable status database.
- Do not push ordinary work directly to `main`.

## Preflight

Before editing, verify the current `main`, applicable instructions, active Issue/PR, overlapping work, relevant architecture/contracts and live protection/check requirements. Treat historical prompts, handovers and migration reports as evidence, not current control-plane truth.

## Validation and merge

- Run repository-selected checks applicable to the changed paths and inspect the complete changed-file set/full diff on the exact final head.
- Preserve authentication, authorization, payment, deployment, durable-data and cross-repository compatibility boundaries.
- Do not weaken the currently required `classify-changes` and `test` checks until a replacement stable `platform-gate` is proven on a representative exact PR head and separately reviewed.
- Security-, authorization-, durable-schema-, deployment-trust-, ruleset/protection- and cross-repository-contract changes require the repository's heightened evidence and any independently required exact-head review.
- Squash merge only after current required checks and reviews pass; delete the merged task branch unless it has a documented continuing provenance role.

## Safety

- Never expose secret values, credentials, private data or sensitive production evidence.
- Production/protected-environment/live-data/credential mutations require separate explicit authority and blast-radius review.
- Do not invoke owner-funded or limited external AI services without explicit owner authorization for that invocation.
- Do not weaken tests, protection, authorization, provenance or deployment controls merely to make a task pass.
- Preserve migration/restore evidence until retention and rollback obligations are explicitly dispositioned.

## Routing

Use `docs/architecture/` and `docs/contracts/` for durable authority, `docs/agents/` for reusable procedures/task packets, `docs/operations/` for operational contracts, and deterministic scripts/workflows for machine-checkable validation. Live GitHub Issue/PR/check/settings state outranks stale status prose.