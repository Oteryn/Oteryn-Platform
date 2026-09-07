# Platform Execution-State Compatibility Note

Organization execution and continuation semantics come from the immutable META policy selected by `docs/agents/META_AGENT_POLICY_BINDING.json`. This file does not define a second retry budget, polling controller or worker lifetime.

META machine states and the local checkpoint schema are different evidence surfaces. Persist local task status using `docs/agents/GOVERNANCE_CONTRACT.json`; classify execution conditions using the bound META states:

- `RUNNING` — useful authorized work can continue;
- `WAITING_EXTERNAL` — an external event is pending and the worker should release capacity;
- `BLOCKED` — authority, safety, permission or a required decision prevents progress;
- `STALLED` — retry/no-progress bounds are exhausted without a new hypothesis;
- `READY` — a concrete safe next action can be resumed;
- `DONE` — acceptance and closeout are complete.

Do not translate an exhausted retry or unavailable tool into missing owner authority. Record the exact condition and one next action. Local numeric values, where still required by a machine consumer, belong in the relevant machine-readable contract rather than copied prose.
