# Branch lifecycle cleanup evidence

Issue: #658  
Parent finding: #586  
Accepted authority: ADR 0024  
Implementation PR: #666  
Implementation merge: `700fa5d0d75a7badd7cb8583d36341c711673942`

## Protected-main apply

Branch Lifecycle run `31081595058` completed successfully on `main`.

- apply artifact: `8959831558`;
- artifact digest: `sha256:391a5a030fa4bfa7c2e0fac197b491925de8006f931577f5318a77de78a91848`;
- deletion timestamp: `2026-08-06T07:43:10+00:00`;
- reviewed and deleted refs: **354**;
- default branch SHA during apply: `700fa5d0d75a7badd7cb8583d36341c711673942`;
- policy SHA-256: `3d646fbd53bd0ae38572dcf80201159d7e05d5562919e55c75f99a83a4031c20`;
- reviewed entries SHA-256: `eeb980e8baab019b592a21712e607f4c27bf8655ccfd5becfb1fd9cdc7cbfa0f`.

The compressed `branch-deletion-evidence.json.xz` file is the exact workflow-produced JSON evidence. After decompression its SHA-256 is:

`e397e0026e882d04fc4e3e5ec5d34e6de68254e63222d6ae9445f32e0cc9f239`

It preserves the exact branch name, head SHA, merged PR number and merge timestamp for every deleted ref.

## Recovery proof

The apply job created `recovery-test/issue-658-31081595058` at the protected-main SHA, deleted it, recreated it at the same SHA, verified restoration and deleted the temporary ref again.

`branch-recovery-test-evidence.json` is the exact workflow evidence; SHA-256:

`e922a5ec498fc649a6fd53ea85f09590fca1e350f99594efd30d69e63607192e`

## Post-cleanup inventory

Read-only Branch Lifecycle run `31082681809` completed on evidence PR #671 after the cleanup and after removal of the one-time approval.

- dry-run artifact: `8960111537`;
- artifact digest: `sha256:9d54b8c6ce7d8ad896f662abdba9090d8d67288bfe1575ce6fa682a745e630d6`;
- branches inventoried: **150**;
- deletion candidates: **0**;
- `UNMERGED_ORPHAN`: 85;
- `UNKNOWN`: 32;
- `OPEN_PR`: 23;
- `ACTIVE_CLAIM`: 9;
- `PROTECTED`: 1 (`main`);
- generated manifest entries: **0** and `apply_on_main=false`.

The compressed `post-cleanup-branch-lifecycle-report.json.xz` file is the exact workflow-produced report. After decompression its SHA-256 is:

`5fde9c4014f2d1827a173fa8c038f2d66e2244c2fd0287e4ece16a47330d5971`

The post-cleanup state intentionally retains unmerged, unknown, open, active and protected refs. They were not part of the reviewed terminal batch and remain fail-closed for separate evidence or ownership resolution.

## Safety result

No deletion was selected by age or prefix. The apply was bound to a reviewed candidate digest and rebuilt the entire live inventory before mutation. `main`, protected refs, open-PR refs, active task/claim refs, deterministic open remediation refs, reserved release/rollback/recovery/backup names, unmerged refs and `UNKNOWN` refs were excluded.

The approval file is removed in the evidence package, so future `main` pushes cannot repeat this one-time cleanup. Any later cleanup batch requires a new live inventory, candidate digest, review and protected-main approval.
