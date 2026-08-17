# Oteryn Platform Short Programme Invocation Registry

```yaml
registry_version: 1.9
repository: blakinio/Oteryn-Platform
trusted_base: main
scope_contract: docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
repair_delivery_contract: docs/agents/REPAIR_PR_ECONOMY.md
remediation_validation_contract: docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
lifecycle_closeout_contract: docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
repair_external_audit_command_status: retired
```

## Purpose

The owner's established short commands remain valid unless explicitly retired below. Resolve work from canonical prompts, immutable scope, current repair/lifecycle contracts, programme state, active tasks, Issues, deterministic branches, PRs, reviews, CI and live ownership.

No command authorizes hidden background execution, production operations, secrets, protected-environment approval or live-data mutation. Commands also do not authorize writes outside `blakinio/Oteryn-Platform` except the explicitly scoped `OTERYN-REPO-MIGRATION` owner alias below. That alias is limited to the repository-migration scope named in its canonical prompt, remains subordinate to every target repository's stricter current `AGENTS.md` hierarchy, and never authorizes writes to `blakinio/canary` or `blakinio/otclient`.

## Ecosystem repository migration

```text
OTERYN-REPO-MIGRATION
Uruchom OTERYN-REPO-MIGRATION autonomicznie.
Kontynuuj OTERYN-REPO-MIGRATION autonomicznie.
Pokaż stan OTERYN-REPO-MIGRATION.
```

```yaml
programme_id: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
canonical_prompt: docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
programme_state: docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
prompt_eval: docs/agents/evidence/OTERYN-20260817-ecosystem-repository-migration-prompt-eval.md
```

The alias is the repository owner's bounded current-task request to verify and, only when proven safe and supported by target-repository governance, execute the physical Oteryn repository migration programme. It covers live topology reconstruction, repository-coordinate inventory, META readiness/bootstrap, `Oteryn-v2` -> `Oteryn-Game` cutover readiness/execution, Platform transfer analysis where required, selective Atlas extraction planning/execution, CI/GHCR/release/provenance impact and rollback. It grants no production/DNS/Synology/secret/live-game mutation and no owner-funded Codex/OpenAI/API use.

### Ultra execution profile

```text
OTERYN-REPO-MIGRATION-ULTRA
Uruchom OTERYN-REPO-MIGRATION-ULTRA autonomicznie.
Kontynuuj OTERYN-REPO-MIGRATION-ULTRA autonomicznie.
Pokaż stan OTERYN-REPO-MIGRATION-ULTRA.
```

```yaml
programme_id: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
canonical_prompt: docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
execution_overlay: docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
programme_state: docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
prompt_eval: docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
execution_budget_class: large
invocation_budget_source: docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
programme_time_limit: none
```

The Ultra alias executes the same canonical migration programme with a stricter high-reasoning execution overlay. The repository-defined large budget applies to one foreground owner invocation only; it is not a lifetime limit for the durable programme. Budget exhaustion requires a recovery-complete checkpoint and accurate rotation/wait/block state, after which a later invocation resumes from durable state. The overlay emphasizes delta-first continuation, hidden-dependency falsification, blocker decomposition, fail-closed Tier-2 cutover and immediate post-mutation verification without expanding repository, production, secret, Synology, DNS, live-game or owner-funded-AI authority.

## Continuous platform audit

```text
Uruchom audyt Platformy autonomicznie.
Uruchom audyt całej Platformy autonomicznie.
Kontynuuj audyt Platformy autonomicznie.
Uruchom OTERYN_PLATFORM_CONTINUOUS_AUDIT autonomicznie.
Kontynuuj OTERYN_PLATFORM_CONTINUOUS_AUDIT autonomicznie.
```

```yaml
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
programme_state: docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
```

The continuous auditor independently inspects the platform, deduplicates problems and creates implementation-authorized Issues when findings are proven. It does not implement product fixes and is not a per-repair merge gate.

## Platform remediation

```text
Uruchom naprawę Platformy autonomicznie.
Kontynuuj naprawę Platformy autonomicznie.
Uruchom OTERYN_PLATFORM_REMEDIATION autonomicznie.
Kontynuuj OTERYN_PLATFORM_REMEDIATION autonomicznie.
```

```yaml
programme_id: OTERYN_PLATFORM_REMEDIATION
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
programme_state: docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
ownership_model: one_issue_one_owner_end_to_end
external_repair_audit: disabled
```

The agent selects or resumes one Issue and owns it end to end:

```text
claim
→ root-cause analysis
→ implementation
→ focused validation
→ exact-head self-review
→ applicable E2E
→ required CI
→ findings remediation
→ merge
→ Issue/task closeout and ownership release
```

It does not stop merely because implementation, PR creation, CI, review or merge completed. It finishes every safe remaining phase in the same Issue-owned workflow.

A different-agent PASS is not required before repair merge.

## Portal completion

```text
PORTAL-CLOSEOUT
Uruchom PORTAL-CLOSEOUT autonomicznie.
Kontynuuj PORTAL-CLOSEOUT autonomicznie.
```

```yaml
programme_id: OTERYN_PORTAL_COMPLETION
canonical_prompt: docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
programme_state: docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
delivery_plan: docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
```

The programme selects or resumes one bounded highest-priority portal-completion slice and owns it through validation and terminal closeout. Existing implementation-authorized audit findings continue through `OTERYN_PLATFORM_REMEDIATION`; the portal-completion programme must reuse that Issue-owned workflow rather than create duplicate repair ownership. The alias grants no production, protected-environment, payment, live-data or external-repository authority.

## Parallel remediation

```text
Uruchom 3 agentów naprawczych Platformy autonomicznie.
Uruchom naprawę Platformy równolegle: 3 agentów.
Kontynuuj równoległą naprawę Platformy autonomicznie.
```

The number means up to that many end-to-end implementation owners. Each agent:

1. selects one distinct eligible Issue;
2. attempts its exact deterministic branch once;
3. selects another eligible Issue after losing a claim;
4. keeps ownership from activation through terminal closeout;
5. normally uses one Issue-owned PR;
6. never waits for another repair worker, train capacity, auditor or integrator slot.

No role is permanently reserved for auditing or integration.

## Retired repair-audit commands

The following historical commands are retired:

```text
Uruchom niezależny audyt gotowych napraw Platformy autonomicznie.
Kontynuuj niezależny audyt gotowych napraw Platformy autonomicznie.
Uruchom audyt napraw Platformy autonomicznie.
```

They must not create an audit-only Issue, frozen audit generation, audit PR, ownership transfer or waiting queue. Interpret them as a request to inspect current repair status unless the owner explicitly requests a new platform-wide audit finding programme.

The separate continuous platform audit command remains available and may open new Issues after independent inspection.

## Exceptional repair train

There is no normal short command that automatically creates repair trains.

An active repair train requires explicit coordinator authorization and is limited to homogeneous low-risk mechanical, documentation, test-fixture or governance work. Ordinary product/runtime repair remains one Issue, one owner and one PR. Terminal lifecycle-only work uses `LIFECYCLE_CLOSEOUT_BATCHING.md`.

## Architecture review

```text
Uruchom przegląd architektury Platformy autonomicznie.
Uruchom audyt architektury Platformy autonomicznie.
Kontynuuj przegląd architektury Platformy autonomicznie.
Uruchom OTERYN_PLATFORM_ARCHITECTURE_REVIEW autonomicznie.
Kontynuuj OTERYN_PLATFORM_ARCHITECTURE_REVIEW autonomicznie.
```

```yaml
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
programme_state: docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
```

The architecture agent analyses and persists proposed or accepted decisions and implementation handoffs. It does not implement runtime code unless separately authorized.

## Status commands

```text
Pokaż stan audytu Platformy.
Pokaż stan napraw Platformy.
Pokaż stan architektury Platformy.
Pokaż stan programów Platformy.
Pokaż stan OTERYN-REPO-MIGRATION.
Pokaż stan OTERYN-REPO-MIGRATION-ULTRA.
```

Status inspection reads programme state, active tasks, Issues, deterministic branches, PRs, reviews and CI without mutation unless continuation is requested.

The historical status command `Pokaż kolejkę wymaganych audytów napraw Platformy` reports that the per-repair audit queue is retired and shows any remaining legacy records requiring cleanup.

## Recovery

`Kontynuuj ...` means:

1. read current scope, contracts, programme state and task checkpoint;
2. verify Issue, implementation owner, branch, PR and live exact head;
3. resume the same Issue-owned role when valid;
4. execute the recorded safe `next_action` immediately;
5. preserve leases, deadlines and recovery counters;
6. use evidence-backed takeover only when ownership is stale;
7. never reconstruct authority from chat memory.

## Status semantics

- `DONE`: the Issue-owned task or selected programme phase is fully terminal.
- `ROTATE`: another eligible task or session should execute a durable next action; it is not an audit handoff requirement.
- `WAITING`: a genuine external dependency, accepted external actor, permission/environment, protected operation, observation window, owner decision or exhausted bounded terminal-CI procedure.
- An agent never uses `WAITING` for another internal worker, auditor or integrator slot.

## Terminal response

```text
STATUS: DONE | WAITING | BLOCKED | ROTATE
RESULT: <observable result>
DURABLE_STATE: <programme/task/Issue/owner/branch/head/PR/validation state>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```