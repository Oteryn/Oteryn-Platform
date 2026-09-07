# Platform Programme Continuation Delta

Use the continuation policy selected by `docs/agents/META_AGENT_POLICY_BINDING.json`. A Platform programme may select work only from its current live selector and Issues; historical plans and prompts are evidence, not a queue.

Before taking a task, verify current ownership and overlap. Keep one mutating owner per writable lane. If one task is `WAITING_EXTERNAL`, persist it and continue only independent authorized work. Do not convert `STALLED`, missing capability or exhausted retries into missing owner permission. Production, protected environments, credentials, live payments and server repositories remain outside the default Platform programme scope.
