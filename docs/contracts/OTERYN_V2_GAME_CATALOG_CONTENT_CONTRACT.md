# Oteryn-v2 Game Catalog Content Contract

Status: Accepted Platform semantic boundary under ADR 0034

Scope: native content authority → Platform `GameCatalog` consumer

Implementation: deferred

## Purpose

This contract defines the Platform-side semantics for consuming authoritative native game-content snapshots without assigning Oteryn-v2 implementation, transport or wire bytes. It is distinct from `GAME_CATALOG_IMPORT_CONTRACT.md`, which remains the Canary compatibility contract.

## Authority split

Oteryn-v2 owns canonical native content identity, executable gameplay definitions and relations, applicability, authoritative source revision/epoch, removal/replacement and whether content is loadable or reachable under a declared ruleset/profile.

Platform `GameCatalog` owns intake policy, validation, immutable imported persistence, candidate review, profile activation/rollback, catalogue projections, presentation localization, administration and audit. An imported row is evidence from the declared authority; Platform persistence does not transfer gameplay ownership.

Wiki owns editorial prose and guides. LiveOps owns current rotations/schedules/runtime observations. Neither may be promoted into deterministic content truth by a catalogue join.

## Semantic snapshot envelope

A native snapshot contract must provide, directly or through a pinned manifest:

```text
contract_id
schema_version
snapshot_id
content_authority_id
authority_epoch
source_revision
generated_at
ruleset_id
content_profile_id
capability_manifest
completeness_manifest
entities[]
relations[]
tombstones[]
payload_digest
```

Names, timestamps and payload hashes are not canonical entity identities. Every entity/relation identity is stable, typed and scoped by the declared native authority. Compatibility IDs may be carried only in namespaced aliases with provenance and can never substitute for native identity.

The producer defines deterministic ordering/canonicalization and digest scope. Platform rejects unsupported versions, duplicate identities, dangling relations, contradictory tombstones, unknown mandatory capabilities, invalid applicability or a digest mismatch.

## Completeness, availability and absence

The envelope distinguishes:

- complete inventory for a declared capability and scope;
- partial or unsupported capability;
- authoritative empty inventory;
- unavailable or stale evidence;
- removed content through an authoritative tombstone;
- conflicting evidence.

Missing data is not a tombstone. An incomplete capability cannot prove absence. Platform public/admin views must not render stale, unavailable or partial evidence as authoritative empty/not-found.

## Imported snapshot lifecycle

1. Receive one immutable artifact from one declared authority profile.
2. Verify bounds, schema, digest, identity uniqueness, relations, applicability and completeness/capabilities.
3. Persist as an inactive candidate with full provenance and findings.
4. Compare against the active snapshot without rewriting either artifact.
5. Activate atomically only for an exact compatible Platform profile after explicit gates.
6. Rebuild projections from the activated snapshot and preserve its identity/revision.
7. Roll back atomically to a retained validated snapshot when its authority epoch remains applicable.

Automatic import never means automatic activation. A restore, replay or delayed artifact cannot lower the active high-water authority revision within the same epoch without an explicit disaster-recovery decision and audit.

## Multiple sources and editorial metadata

One active profile has one declared authoritative gameplay-content source. Native, Canary and historical snapshots may coexist for comparison or different profiles, but records from different authorities are not combined field by field into a single authoritative entity.

Reviewed Platform metadata may supplement only allowlisted presentation fields. Each supplemented field carries its source/revision and an explicit precedence rule. It cannot change identifiers, executable parameters, relations, availability, completeness, tombstones or ruleset applicability.

## Legacy Canary Compatibility

`GAME_CATALOG_IMPORT_CONTRACT.md`, schemas `1.0.0`–`1.3.0`, programme #330 and draft PR #338 remain current Canary compatibility artifacts. Their final-registry assumptions and canonical keys remain correct only for that source profile.

Compatibility adapters must:

- declare `legacy-canary` authority and exact schema/source revision;
- keep Canary aliases and provenance namespaced;
- fail closed on unsupported producer/consumer combinations;
- preserve inactive import, transactional activation and rollback;
- never present compatibility availability as native authority;
- never silently fall back after a native profile has selected native authority.

## Cutover and rollback

Rollout is consumer-first: register support, import inactive, validate, shadow/diff, prove mixed-version behavior, then activate one exact profile. Cutover requires an explicit authority switch and prevents concurrent co-authoritative publication.

Rollback disables new native activation and restores a retained compatible snapshot/profile while preserving provenance and operator visibility. It must not merge newer native facts into an older Canary snapshot, discard tombstones or claim that compatibility fallback is native truth.

## Security and operations

- producer identity and artifact integrity are authenticated outside the browser;
- public/admin upload remains forbidden unless separately designed;
- artifact size/count/depth and text fields are bounded;
- activation/rollback require exact permission, confirmed MFA and durable audit;
- raw rejected payloads and sensitive operational paths do not enter ordinary logs;
- direct shared-database reads/writes are not the native steady-state boundary.

## Deferred implementation obligations

The external producer owner must separately select exact identity forms, capability taxonomy, revision/epoch allocation, canonical serialization, signing/authentication, transport, retention and runtime collection point. No Platform document may invent those bytes or claim external implementation exists.

## Acceptance invariants

1. Platform never becomes authoritative for executable gameplay content by importing it.
2. Native identity never depends on Canary numeric IDs, table names or schema keys.
3. One profile never exposes records from two co-authoritative content sources.
4. Partial/stale/unavailable evidence never becomes authoritative absence.
5. Activation and rollback are exact-snapshot, transactional and auditable.
6. Delayed/replayed evidence cannot silently lower accepted authority revision.
7. Compatibility remains explicit and removable without changing the native contract.
