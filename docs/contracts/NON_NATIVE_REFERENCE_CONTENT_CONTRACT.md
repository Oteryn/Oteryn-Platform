# Non-native Reference Content Contract

Status: Accepted Platform semantic boundary under ADR 0042

Contract ID: `oteryn.reference-content`

Scope: provenance-pinned non-native source material → Platform `ReferenceContent` → explicit reference consumers

Implementation: deferred

## Purpose

This contract defines how Oteryn Platform may normalize and consume structured facts from a source artifact that is useful for comparison/reference but is **not authoritative gameplay content**.

It is separate from:

- `OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md`, which defines the native Oteryn-v2 → Platform GameCatalog authority boundary;
- `GAME_CATALOG_IMPORT_CONTRACT.md`, which defines Legacy Canary Compatibility import semantics;
- Wiki editorial publication lifecycle;
- any third-party-rights/publication decision.

A conforming reference artifact grants no runtime, deployment, producer, native-identity or publication authority.

## Authority class

Every artifact and projection carries:

```text
source_evidence_class = NON_AUTHORITATIVE_REFERENCE
```

This class is immutable for the lifetime of the reference snapshot. Re-review may create a newer snapshot or independently verify a crosswalk, but it cannot mutate the original reference source into native or `legacy-canary` authority.

`ReferenceContent` never participates in `GameCatalog` active authority-profile selection or automatic fallback.

## Initial source instance

The owner-supplied source archive currently known to Issue #1115 is:

```text
source_artifact_name = crystalserver-main.zip
source_artifact_sha256 = 920a59e15175a5f53721f60b17f4bb37370bf0b61cd91abb4c909bf0d85e5f26
```

Observed source profiles include:

```text
data-global
data-crystal
```

They are separate profiles. This contract does not declare them semantically equivalent and does not authorize combining them.

The contract is generic: later source artifacts require their own pinned identity and review; they do not inherit trust merely because this initial archive is allowed as reference material.

## Reference snapshot envelope

Every immutable reference snapshot provides:

```text
reference_contract_version
reference_snapshot_id
source_evidence_class
source_kind
source_artifact_name
source_artifact_sha256
source_profile_id
fact_family
source_paths[]
extractor_id
extractor_version
transformation_digest
extracted_at
coverage_scope
completeness_state
semantic_state
review_state
facts[]
relations[] optional
payload_digest
```

Optional evidence may include source-side version/revision metadata only when it is present in the pinned artifact and represented without interpretation. An absent source revision remains unknown.

`extracted_at` is processing provenance, not gameplay freshness.

## Source-path and profile provenance

Every fact is traceable to one or more sanitized relative source paths inside the pinned artifact. Absolute host paths, credentials, URLs requiring network fetches and mutable external locations are forbidden.

A snapshot binds exactly one `source_profile_id`. Cross-profile analysis is performed as a comparison between immutable snapshots, never by constructing a synthetic merged profile.

## Fact-family classification

The source pipeline assigns one semantic class per emitted fact family:

- `DIRECT_STRUCTURED` — deterministic field extraction is possible;
- `TRANSFORM_REQUIRED` — deterministic parsing/normalization is required and identified by extractor/transformation version;
- `PARTIAL_SEMANTICS` — source material cannot prove complete meaning required for authoritative/current claims;
- `EDITORIAL_ONLY` — research lead only; not emitted as a structured reference fact for automatic player-facing use;
- `AUTHORITY_REQUIRED` — technically extractable but the requested consumer use requires authority not provided by this contract;
- `REJECTED` — unsafe, ambiguous, incompatible or prohibited.

`DIRECT_STRUCTURED` describes extractability, not authority. `TRANSFORM_REQUIRED` describes processing, not permission to execute source code.

## Static, non-executable extraction

Reference extraction treats all source material as untrusted data.

Allowed:

- XML/JSON/text parsing;
- bounded static parsing of Lua or other source forms;
- deterministic normalization from source syntax into explicit data;
- bounded cross-file resolution where every input path is inside the pinned artifact and the transformation is deterministic;
- generated test fixtures and reviewable normalized artifacts.

Forbidden:

- executing source Lua/PHP/JavaScript or binaries to obtain facts;
- performing network fetches during extraction;
- invoking arbitrary build scripts from the source archive;
- reading machine-local mutable state as implicit input;
- silently filling unknown fields with defaults from another source/profile.

If exact semantics require execution or unavailable runtime context, classify the affected material `PARTIAL_SEMANTICS` or `AUTHORITY_REQUIRED` and fail closed for consumers that require exact semantics.

## Identity

### Reference identity

Each emitted reference entity has a deterministic source-local identity equivalent to:

```text
(reference_source_id,
 source_artifact_sha256,
 source_profile_id,
 fact_family,
 source_local_key)
```

The exact serialization is an implementation detail, but all dimensions are preserved.

Names, display labels, source paths, source numeric IDs and payload hashes do not become native Oteryn identities by themselves.

### Authority crosswalk

A separate reconciliation record may bind a reference entity to an independently authoritative target:

```text
reference_entity_id
target_authority_id
target_entity_id
mapping_state = CANDIDATE | VERIFIED | REJECTED
mapping_evidence
reviewed_at
```

Rules:

- `CANDIDATE` may come from deterministic matching heuristics and is never authoritative;
- `VERIFIED` requires evidence from the target authority or an accepted target-side mapping source;
- `REJECTED` preserves a disproven candidate without recycling it;
- a crosswalk does not copy target authority to the reference entity;
- consumers may join to authoritative data only through an allowed `VERIFIED` mapping and must preserve which fields came from which source.

## Completeness

Every snapshot declares exactly one:

```text
COMPLETE_FOR_DECLARED_SCOPE
PARTIAL
UNKNOWN
```

The declared scope includes source artifact, profile, fact family and any path/filter bounds.

Rules:

- complete extraction of the declared source scope is not completeness of Oteryn gameplay content;
- missing data in `ReferenceContent` is never an authoritative tombstone or proof of nonexistence;
- partial/unknown coverage cannot be rendered as authoritative empty/not-found;
- source-side inventory counts are not Platform production population counts.

## Semantic and review state

Every emitted snapshot records a semantic state, for example:

```text
STRUCTURED
TRANSFORMED
PARTIAL_SEMANTICS
```

and a review state, for example:

```text
UNREVIEWED
STRUCTURALLY_VALIDATED
SEMANTICALLY_REVIEWED
REJECTED
```

A structurally valid parser output may still be semantically unsuitable for a consumer. Consumer admission checks both dimensions.

## Freshness and applicability

A pinned reference snapshot is reproducible, not automatically current.

Unless independently proven, applicability to the active Oteryn ruleset/profile is:

```text
UNKNOWN
```

No consumer may infer currentness from:

- archive filename;
- extraction timestamp;
- source commit-like metadata;
- record count;
- similarity to native names or identifiers;
- the fact that the archive was owner-supplied.

A later comparison to authoritative data may establish a bounded relationship for exact mapped facts, but does not make the whole reference snapshot current.

## Conflict and precedence

### Reference versus native

For a native Oteryn claim, authoritative Oteryn-v2 evidence wins. The reference value may remain visible only as explicitly labelled comparison/reconciliation evidence.

If native evidence is unavailable, the state remains unavailable/unknown; do not promote the reference value to native truth.

### Reference versus Legacy Canary Compatibility

Legacy Canary Compatibility is governed only by `GAME_CATALOG_IMPORT_CONTRACT.md`. Reference data never uses the `legacy-canary` authority name and does not inherit Canary schema/identity semantics.

### Reference versus reference

Conflicting source artifacts/profiles remain separate. The Platform may produce a diff/finding, but may not synthesize a winner without a separately accepted precedence rule.

## Consumer contract — offline reconciliation

Allowed uses:

- compare inventories and normalized fields;
- detect candidate missing/changed mappings;
- generate crosswalk candidates;
- test extractor determinism;
- measure source-family coverage;
- prepare migration/reconciliation reports.

Required output preserves both snapshot identities and classifies differences as findings, not mutations.

Offline reconciliation may never write authoritative GameCatalog facts or activate a profile automatically.

## Consumer contract — Wiki structured reference

Wiki may consume a bounded reference projection when:

- the projection exposes `NON_AUTHORITATIVE_REFERENCE` status;
- source artifact/profile/snapshot provenance remains available to the presentation layer;
- unknown/partial/conflicting state is rendered explicitly;
- the page or section cannot be mistaken for current/native authoritative data;
- an authoritative equivalent, when available, is presented as the authoritative source rather than being overwritten by reference values;
- source-local identity is not published as native identity unless an independently `VERIFIED` crosswalk supplies the native identifier;
- publication rights for the exact material are separately satisfied.

Generated structured facts and editorial Wiki prose remain different content classes. This contract does not authorize automatically creating editorial narrative, translations or strategy advice from source text.

Descriptions, dialogue, quest text, maps, images, sprites, media and other expressive source material remain outside this contract's publication authority even when they are technically parseable.

## Consumer contract — PlayerCompanion

PlayerCompanion may consume `ReferenceContent` only through a bounded application/query projection; it must not parse the raw archive itself.

Every materially reference-dependent persisted or displayed result records:

```text
source_evidence_class = NON_AUTHORITATIVE_REFERENCE
reference_snapshot_id
source_profile_id
fact_family
assumptions
limitations
```

The existing `DETERMINISTIC | SIMULATION | RECOMMENDATION` result classification remains unchanged.

Rules:

- a material gameplay parameter supported only by reference evidence cannot back a result presented as current authoritative `DETERMINISTIC` Oteryn gameplay truth;
- the workflow either obtains authoritative GameCatalog/ruleset evidence or presents the result as a simulation/reference/recommendation with visible limitations;
- reference data may support exploration, planning candidates, comparisons, checklists and research-oriented views where uncertainty is explicit;
- missing reference data cannot imply inaccessible, unavailable, completed, impossible or absent game state;
- reference evidence cannot establish character/world/runtime state;
- current gameplay availability/reachability remains outside this source class;
- no automatic switch from unavailable authoritative data to reference data is allowed.

## Publication-rights boundary

This contract is an architecture/data-semantics decision, not a legal-rights determination.

ADR 0026 and `THIRD_PARTY_NOTICES.md` remain controlling for third-party material. Before a release or public projection includes third-party-derived material, the delivery owner must inventory the exact included material, identify applicable terms and satisfy the repository provenance/distribution gate.

Unknown or incompatible rights are a restriction. This contract never turns source possession, a root license file or factual normalization into a blanket permission to republish expressive material.

## Security and resource bounds

Any implementation must enforce at least:

- configured maximum archive and expanded-byte limits;
- maximum file count, path length, nesting/depth and per-file size;
- zip-slip/path traversal rejection;
- symlink/special-file handling policy;
- deterministic text decoding and malformed-input rejection;
- bounded entity/relation/string/array counts;
- no external network/resource fetches;
- no source-code execution;
- no secrets, credentials, private keys or personal data in generated public artifacts;
- rejected raw payloads and host paths excluded from ordinary logs.

## Lifecycle

A reference snapshot may use lifecycle states equivalent to:

```text
pending
validating
validated
rejected
deprecated
```

There is deliberately no authority-bearing `active` state equivalent to GameCatalog profile activation.

A consumer may explicitly select one validated reference snapshot for one operation or product slice. Selection is part of the consumer configuration/request and the selected snapshot identity is emitted in the result; it does not become a global gameplay authority alias.

Deprecation prevents new consumption while retaining historical reproducibility and audit evidence.

## Failure semantics

Reject or mark unavailable for the requested consumer when any of these is true:

- artifact digest mismatch;
- unsupported contract/extractor version;
- invalid profile/path selection;
- nondeterministic extraction;
- malformed source exceeding bounds;
- unresolvable required relation;
- partial semantics where the consumer requires exact semantics;
- missing provenance dimension;
- conflicting facts that the consumer would have to blend;
- source status would be hidden or presented as authoritative;
- requested public material lacks the required provenance/rights decision.

A failure never mutates or rolls back an authoritative GameCatalog snapshot.

## Acceptance invariants

1. `ReferenceContent` never becomes native Oteryn or Legacy Canary Compatibility authority.
2. No reference snapshot participates in authoritative GameCatalog activation/fallback.
3. Every fact remains traceable to pinned artifact, profile, source path and transformation version.
4. `data-global` and `data-crystal` remain separate source profiles unless a later accepted decision proves compatibility.
5. Source-local identity never substitutes for native identity.
6. Missing/partial/conflicting reference evidence never becomes authoritative absence or a fabricated default.
7. Source code is parsed as untrusted data and is never executed to derive reference facts.
8. PlayerCompanion and Wiki preserve `NON_AUTHORITATIVE_REFERENCE` classification through their outputs.
9. Native authority unavailability never silently promotes reference evidence.
10. Third-party expressive-content publication rights remain independently gated.

## Deferred implementation obligations

A later Platform implementation task must choose exact persistence/schema names, serialization, extractor packaging, normalized fact schema, review/admin workflow, consumer query interfaces, retention and UI presentation. Those choices must preserve this contract and may reuse generic immutable-artifact infrastructure without reusing GameCatalog authority activation semantics.

No implementation is authorized by this document.
