---
task_id: OTERYN-20260808-www-only-scope-guard
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - Platform PR #931
optional_reads: []
---

# OTERYN-20260808 WWW-only repository scope guard — archived

## Terminal status

```yaml
status: completed
repository: blakinio/Oteryn-Platform
delivery_pr: 931
delivery_head: 7460f35ab8cc7f31eb55a1326f7b2096fc00c83c
merge_commit: 59e55aecf30c1bd2bf88423d01ab1f3d5d86b3ea
merged_at: 2026-08-08T21:55:33+02:00
ownership_released: true
runtime_e2e: NOT_APPLICABLE
```

## Outcome

The project-owner scope rule is canonical on protected `main` in `AGENTS.override.md`.

For Platform work, the default authorized repository is **only** `blakinio/Oteryn-Platform`.

Server/game repositories, including `blakinio/Oteryn-v2` and repositories whose primary responsibility is game server, runtime, gameplay protocol or server persistence, must not be accessed, read, inspected, searched, fetched, branched, edited, reviewed, audited, merged or otherwise operated on unless the owner first grants explicit separate permission for server-repository work.

Generic commands such as `dzialaj dalej`, `kontynuuj`, autonomous continuation, audit, repair, architecture continuation or implementation do not expand Platform scope into server repositories.

If Platform work appears to require server-side evidence, the agent must stop before accessing the server repository and request explicit owner permission.

## Validation

- exact delivery head: `7460f35ab8cc7f31eb55a1326f7b2096fc00c83c`;
- Agent Governance run `31275616033`: PASS;
- CI run `31275616038`: PASS;
- PR #931 merged to protected `main` as `59e55aecf30c1bd2bf88423d01ab1f3d5d86b3ea`;
- runtime/component E2E: `NOT_APPLICABLE` because this is governance/documentation only;
- no server repository access or mutation is authorized by this closeout.

## Acceptance criteria

- [x] Root agent bootstrap contains WWW-only default scope.
- [x] Explicit separate owner permission is required before any server-repository access or mutation.
- [x] Generic continuation commands cannot expand Platform scope into server repositories.
- [x] Required exact-head governance and CI passed.
- [x] PR #931 merged to protected `main`.
- [x] Task archived and ownership released.
