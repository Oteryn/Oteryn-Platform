All copied files are byte-exact from **Oteryn/Oteryn-Game** at
`3b49a20feec3ba29700bb4b2fa28fa9b539b79a2` (PR #3010, CANDIDATE rev 1).
Only raw URLs at this full SHA were fetched. No Game repository writes occurred.

| Local file | Source path |
| --- | --- |
| snapshots.json | docs/contracts/fixtures/world-event-guild-season/snapshots.json |
| plat3_authority.proto | docs/contracts/game-events/v1/plat3_authority.proto |
| GAME_EVENT_FOUNDATION_REGISTRY.json | docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json |
| CROSS_REPOSITORY_CONTRACT_LOCK.json | docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json |

Unit tests pin SHA-256 of all four complete files (including final LF), every
exact embedded wire string (excluding JSON framing/LF), and provided record hashes.
There are **no producer signatures** in these Game fixtures. Tests use a public,
synthetic Ed25519 test seed to sign the unchanged wire strings, verify them, and
reject mutated bytes, missing signatures and wrong keys. This proves the local
verification utility, not Game PKI or producer/consumer runtime conformance.
