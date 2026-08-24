# OTERYN v3.10 Platform Documentation/Agent IA closeout evidence

Alias: `OTERYN-V310-PLATFORM-DOC-IA-CLOSEOUT`
Governing Issue: #1260
Implementation PR: #1261
Audited protected-main baseline: `b930d2782e1d2fe01f66cde5c28b1c2486541cec`

## Inventory evidence

- Prompt root: `docs/agents/prompts` — exact inventory 22 Markdown files.
- Classification: 10 `reusable` / `active_reusable` / executable; 12 `one_shot_historical` / `historical_do_not_run` / non-executable.
- Handover root: `docs/agents/handovers` — exact inventory 3 Markdown files; all cataloged `authoritative: false`.
- Effective Documentation/Agent IA instruction chain: `AGENTS.md` then `docs/agents/AGENTS.md`.
- Measured absent nearer overrides: `docs/agents/prompts/AGENTS.md`, `docs/agents/handovers/AGENTS.md`, `tools/agents/AGENTS.md`, `.github/AGENTS.md`.

## Task-lifecycle evidence

- Issue #864 is terminal/closed while `OTERYN-20260805-native-auth-production-verification` remained active on the audited baseline; the stale cache is archived and its original full body remains recoverable at baseline blob `f96872f1d1c2b3b96db518ce348a910dcad83b7a`.
- Issue #91 is open and is recorded as the governing Issue for the surviving public-domain active task.
- Policy revision 4 adds fail-closed active-task governing-Issue liveness while retaining checkpoint structure version 1.

## Deterministic prevention

- `tools/agents/documentation_ia.py` fails closed on prompt/handover inventory drift, lifecycle metadata contradictions, or unmeasured nearer instruction overrides.
- `tools/agents/task_issue_liveness.py` fails closed when an active task lacks a numeric governing Issue, the Issue cannot be read, the identity is a PR, or the Issue is terminal.
- Agent Governance grants only `issues: read` in addition to existing read permissions and runs both new unit suites plus both live/deterministic validators.

## Verified candidate evidence

Implementation candidate `dabd49be0895c975dc39e7a87a2da7e722ee2a10`:

- Agent Governance run `32756637341`: `SUCCESS`.
- CI run `32756637270`: `SUCCESS`; required `platform-gate` job `97525698371`: `SUCCESS`.
- CI routing classified runtime tests and PHP coverage as skipped/not applicable for this Documentation/Agent IA-only change.

The final merge candidate adds only this evidence receipt plus the merge-bound archive of the closeout task and removes the active closeout cache. Its exact-head check suite and merge result are recorded on PR #1261 and verified live after merge to avoid invalidating that candidate with a post-check documentation commit.

## Scope

No product runtime, deployment, database/migration, runner, package/release, production, ruleset/branch-protection, secret, dependency, Recovery-organization, or external-repository mutation is part of this closeout.

Out-of-scope observation retained without mutation: `docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md` still contains migration-programme state/pointers that are separate from this bounded Documentation/Agent IA closeout. The v3.10 prompt explicitly forbids reopening migration here.
