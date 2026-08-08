# OTERYN-20260808 WWW-only repository scope guard

```yaml
task_id: OTERYN-20260808-www-only-scope-guard
mode: GOVERNANCE
status: ready
repository: blakinio/Oteryn-Platform
base_branch: main
branch: docs/OTERYN-20260808-www-only-scope-guard
owner: GPT-5.6 Sol
created_at: 2026-08-08T21:48:00+02:00
owned_paths:
  - AGENTS.override.md
  - docs/agents/tasks/active/OTERYN-20260808-www-only-scope-guard.md
```

## Owner instruction

Default work scope is **WWW Platform only**: `blakinio/Oteryn-Platform`.

Server/game repositories, including `blakinio/Oteryn-v2` and any other server/runtime/protocol repository, are **read-only unless the project owner explicitly asks for server work and grants separate write authorization**.

Generic commands such as `dzialaj dalej`, `kontynuuj`, autonomous continuation, architecture continuation, audit, repair or implementation while operating in the Platform project do **not** authorize server-repository writes.

Cross-repository evidence may be read when needed to keep Platform contracts compatible, but no server branch, file, PR, issue, merge, runtime or configuration may be mutated without explicit owner authorization naming server work.

This guard exists because an FND-04 server architecture task was accidentally started during a Platform-only session. That server task was checkpointed separately and must not be resumed implicitly.

## Acceptance

- [x] Persist owner WWW-only default scope.
- [x] Require explicit separate authorization before any server-repository write.
- [x] Preserve read-only cross-repository architecture evidence for Platform compatibility.
- [x] Do not expand authority beyond `blakinio/Oteryn-Platform`.
