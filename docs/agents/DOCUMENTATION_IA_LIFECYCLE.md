# Oteryn Platform Documentation/Agent IA Lifecycle

## Authority

`docs/agents/DOCUMENTATION_IA_CATALOG.json` is the deterministic classification index for the retained prompt and handover libraries. It does not replace current system/owner instructions, the `AGENTS.md` hierarchy, live GitHub Issue/PR state, or canonical programme state. It only answers whether a retained prompt is currently executable/reusable and how a retained handover must be treated.

Every change under `docs/agents/prompts/*.md` or `docs/agents/handovers/*.md` must update the catalog in the same change. `python tools/agents/documentation_ia.py` fails closed when the filesystem inventory and catalog diverge.

## Prompt lifecycle

Each retained prompt has one stable catalog `id`, `version`, `owner`, `scope`, `classification`, `status`, `executable`, `supersession`, and `provenance`.

- `reusable` + `active_reusable` + `executable: true`: the prompt may be invoked only through current trusted authority and must reconstruct live state at invocation time. Embedded Issue, PR, SHA, branch, owner, or repository examples never override live authority.
- `one_shot_historical` + `historical_do_not_run` + `executable: false`: the file is retained only so historical references and provenance resolve. It grants no current execution authority. A new task must route through the cataloged supersession target or current governance instead of restarting the historical prompt.

The catalog status is authoritative for prompt executability even when a historical file body predates this lifecycle model and still contains imperative prose.

## Handover lifecycle

Retained handovers are evidence snapshots only and always have `authoritative: false`.

A handover is either:

- `expired`: historical context only; its next actions and status statements are not executable current state; or
- `supersede_on_live_transition`: a frozen handoff usable as evidence until a newer live Issue, PR, task, branch/head, programme, or protected-main transition changes the referenced state.

On every continuation, reconcile the handover against live GitHub and current protected-main governance before using it. A stale handover never keeps an Issue open, keeps a task active, owns a branch/path, or proves a PR/CI status.

## Task authority

Active task packets are durable caches, not lifecycle databases. Every active task must declare a top-level numeric `governing_issue`. The governing GitHub Issue must be open. The live PR/branch validator remains responsible for PR and source-branch ownership; `task_issue_liveness.py` independently fails closed when the governing Issue is missing, cannot be read, is actually a PR, or is terminal.

When a governing Issue closes, its task must leave `docs/agents/tasks/active/` in the same bounded closeout flow. Historical evidence belongs in `docs/agents/tasks/archive/`.

## Instruction chain measurement

For the current Documentation/Agent IA surfaces the effective repository instruction chain is:

1. `AGENTS.md`
2. `docs/agents/AGENTS.md`

The catalog also records measured-absent nearer overrides. If a nearer `AGENTS.md` is introduced later, the deterministic IA validator fails until the catalog is deliberately re-measured and updated.

## Validation

Run:

```sh
python tools/agents/documentation_ia.py
python tools/agents/test_documentation_ia.py
python tools/agents/test_task_issue_liveness.py
```

Agent Governance also runs the catalog validator and live governing-Issue check on every matching PR.
