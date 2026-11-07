Source: Oteryn/Oteryn-Game
Full SHA: 3b49a20feec3ba29700bb4b2fa28fa9b539b79a2 (PR #3010, CANDIDATE revision 1)
Path: docs/contracts/fixtures/account-deletion/closure.json
closure.json is byte-exact, including its trailing LF. The fixture's `wire` strings
exclude JSON fixture framing/LF. All four exact wires and their supplied SHA-256
hashes are tested. This pinned family contains one fixture file.

The source has no Ed25519 signatures or signing-key fixture. Tests sign these exact
Game fixture bytes with an explicitly synthetic local test key and verify them,
including tampering/wrong-key failures. No synthetic key is deployment trust and
no signature field or new envelope has been added to the candidate wire.

Lifecycle source: Oteryn/Oteryn-Game
Full SHA: 9668e0a04e1a631ebfc8752772102a1fd98ce8fa
Path: docs/contracts/fixtures/account-deletion/lifecycle.json
Lifecycle fixture copied byte-exactly, including trailing LF.
