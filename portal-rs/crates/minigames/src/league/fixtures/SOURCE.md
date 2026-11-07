All copied files are byte-exact from **Oteryn/Oteryn-Game** at
`9668e0a04e1a631ebfc8752772102a1fd98ce8fa` (PR #3015, T94 producer).
Only raw URLs at this full SHA were fetched. No Game repository writes occurred.

| Local file | Source path |
| --- | --- |
| snapshots.json | docs/contracts/fixtures/world-event-guild-season/snapshots.json |
| plat3_authority.proto | docs/contracts/game-events/v1/plat3_authority.proto |
| GAME_EVENT_FOUNDATION_REGISTRY.json | docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json |
| CROSS_REPOSITORY_CONTRACT_LOCK.json | docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json |

Unit tests pin SHA-256 of all four complete files (including final LF), every
exact embedded wire string (excluding JSON framing/LF), and provided record hashes.
There are **no producer signatures** in these Game fixtures or the T94 HTTP wire.
Production admission pins the verified dedicated mTLS client leaf SPKI SHA-256.
Local adapter audit records retain canonical JSON and that verified identity;
they are not Game ANL envelopes. Native ANL evidence remains on Game's side.
Synthetic Ed25519 utility tests do not define the publisher wire or qualify PKI.
