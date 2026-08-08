# OTERYN-20260808 WWW-only repository scope guard

```yaml
task_id: OTERYN-20260808-www-only-scope-guard
mode: GOVERNANCE
status: validating
repository: blakinio/Oteryn-Platform
base_branch: main
branch: docs/OTERYN-20260808-www-only-scope-guard
owner: GPT-5.6 Sol
created_at: 2026-08-08T21:48:00+02:00
updated_at: 2026-08-08T21:50:00+02:00
owned_paths:
  - AGENTS.override.md
  - docs/agents/tasks/active/OTERYN-20260808-www-only-scope-guard.md
```

## Owner instruction

Default work scope is **WWW Platform only**: `blakinio/Oteryn-Platform`.

Server/game repositories, including `blakinio/Oteryn-v2` and any other server/runtime/protocol repository, must **not be accessed, read, searched, fetched, audited or modified unless the project owner first grants explicit separate permission for server-repository work**.

Generic commands such as `dzialaj dalej`, `kontynuuj`, autonomous continuation, architecture continuation, audit, repair or implementation while operating in the Platform project do **not** authorize any server-repository operation.

If Platform work appears to require server-side evidence, stop before accessing that repository and request explicit permission from the project owner.

This guard exists because an FND-04 server architecture task was accidentally started during a Platform-only session. The owner explicitly allowed one final server checkpoint recording that the already-created FND-04 changes must later be completed and independently audited. That checkpoint does not authorize any further server access.

## Acceptance

- [x] Persist owner WWW-only default scope in root agent bootstrap.
- [x] Require explicit separate authorization before **any** server-repository access or mutation.
- [x] Generic continuation commands cannot expand Platform scope into server repositories.
- [x] Do not expand authority beyond `blakinio/Oteryn-Platform`.
- [ ] Merge this governance change to protected `main` so future Platform invocations inherit it from the trusted base.

## Next action

Validate PR #931 and merge when protected checks permit, then archive this governance task. No server repository operation is authorized by this task.
