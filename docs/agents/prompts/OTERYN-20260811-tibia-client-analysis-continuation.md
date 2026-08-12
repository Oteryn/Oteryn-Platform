# Continuation prompt — OTERYN-20260811 Tibia client analysis

Copy the prompt below verbatim into a fresh agent session.

---

Continue task `OTERYN-20260811-tibia-client-analysis` autonomously from durable repository state.

Repository: `blakinio/Oteryn-Platform`
Branch: `ops/oteryn-tibia-client-analysis-20260811`
PR: `#1006` (draft at the last durable checkpoint)

Before any mutation, read and follow the repository's applicable governance, especially:

1. `docs/agents/PROMPTING_STANDARD.md`
2. `docs/agents/PROMPTING_HANDOVER.md`
3. `docs/agents/CONTEXT_HANDOFF.md`
4. `docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md`
5. `docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md`
6. `docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md`

Treat live GitHub/repository/runtime state as authoritative. Verify current branch, exact HEAD, PR state, CI, open-PR/path ownership, current workflow runs, runner state, and runtime ownership before mutation. Do not reconstruct the investigation from chat. Do not repeat rejected hypotheses unless new evidence contradicts the durable checkpoint.

Primary objective:

Finish reverse-engineering the decoded Tibia Worldmap message path and prove one real decoded map capture equivalent to:

`(x, y, z) -> ordered tile/field contents -> appearance/type IDs`

Already PROVEN and not to be rediscovered without contradictory evidence:

- verified client `/data/client-15.32.df7b29/bin/client`
- version `15.32.df7b29`
- size `51965216`
- SHA-256 `e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe`
- Worldmap QMetaObject `0x3087800`
- Worldmap `static_metacall=0xdf2a60`
- Worldmap jump table `0x1d8bd10`
- `handleFullMapMessage -> 0xcec8d0`
- `handleFieldDataMessage -> 0xcd3190`
- `handleCreateOnMapMessage -> 0xcecc70`
- `handleChangeOnMapMessage -> 0xcecf40`
- `handleDeleteOnMapMessage -> 0xcd4e20`
- FieldData and multiple directional map-update paths converge on `0x19a8a80`
- protobuf `Coordinate`: `x=1:uint32`, `y=2:uint32`, `z=3:uint32`
- `0xde9ca0` is `tibia::sessiondump::TSessiondumpPlayer`, NOT Worldmap
- encrypted TCP is not the starting point while the decoded protobuf route remains viable
- whole-.text Python/Capstone scanning is rejected on this runner; prefer bounded/native streaming analysis

Current high-confidence INFERENCES that still need direct proof where relevant:

- `0x19a8a80` is the central decoded map-field application/iteration routine
- `0x313a820` is a Coordinate default-instance candidate
- `0x313a860` is a MapFieldData default-instance candidate
- `0x314b480` is an AppearanceInstance default-instance candidate

Owned runtime environment:

- runner label: `oteryn-staging`
- verified runner: `oteryn-synology-staging`
- container: `oteryn-tibia-client-analysis`
- persistent data: `/volume1/docker/oteryn/tibia-analysis`
- client: `/data/client-15.32.df7b29/bin/client`
- required ownership labels: `com.blakinio.owner=oteryn`, `com.blakinio.purpose=tibia-client-analysis`

Safety boundary:

- Do not modify, restart, stop, recreate, clean or otherwise disturb canonical `oteryn-staging` services.
- Do not perform blanket Docker cleanup.
- Verify ownership labels before modifying/restarting the analysis container.
- Do not commit Tibia binaries, extracted proprietary assets, credentials, tokens, session material, account information or protected character data.
- Preserve only safe hashes, addresses/layout findings, scripts, bounded evidence and reproducible instructions.

Exact work sequence:

1. Inspect live PR #1006 HEAD/CI and all relevant completed/pending Tibia analysis workflow runs. Persist any completed trace evidence that is not yet in the reports. Do not claim queued/in-progress runs succeeded.
2. Continue bounded static reverse of `0x19a8a80` until field iteration/content ordering and appearance/type extraction are understood well enough to instrument deterministically.
3. Tie the default-instance candidates to concrete generated protobuf types where evidence permits.
4. Identify the lowest-risk interception point after `TProtobufServerMessageTranslator` and before mutation of `TWorldMapStorage`. Prefer the already-decoded Worldmap boundary over raw network capture.
5. Determine whether the owned runtime already has a safe authenticated-session/test-account mechanism. Inspect without printing or persisting secrets.
6. A real FullMap/FieldData runtime capture likely requires an active game-world session. Do NOT ask for or accept credentials in chat/logs/scripts. If an existing safe session mechanism is available, use it without exposing secrets.
7. If no safe session exists and interactive authentication is genuinely required, checkpoint every static result first, then stop only at that user-authority boundary and give the user the minimum precise action needed to establish a session locally/interactively. Do not ask them to paste a password or token.
8. Once an authenticated session exists, perform one bounded capture of an actual decoded map message and prove deterministic output equivalent to `(x,y,z) -> ordered contents -> appearance/type IDs`.
9. Validate that canonical staging is unchanged before and after runtime work.
10. Update the active task record and reports after every material discovery. Maintain `PROVEN`, `DERIVED/INFERENCE`, `UNKNOWN`, `CONFLICT`, rejected hypotheses, validation evidence and exactly one accurate `next_action`.
11. Do not stop merely because a commit, workflow run or checkpoint was created. Continue until the decoded-map capture objective is proven or a real authority/blocker boundary is reached.
12. Before terminal closeout, remove/reduce temporary one-shot/trace workflows that should not be merged, according to repository governance, without deleting the retained proprietary runtime data unless the task explicitly no longer needs it.

Expected completion evidence:

- concrete interception address/path and rationale;
- exact decoded message used (`FullMap`, `FieldData`, or another relevant map update);
- bounded capture evidence from the verified client;
- deterministic records containing coordinates, ordered contents and appearance/type identifiers;
- validation showing canonical `oteryn-staging` unchanged;
- durable task/report checkpoint and clean PR state.

Do not declare completion without the real decoded-message capture proof.
