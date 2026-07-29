# ADR 0018: Represent an unknown Game Catalog verified boundary explicitly

Status: Accepted  
Date: 2026-07-29

## Context

Game Catalog schema `1.0.0` requires `snapshot.verified_content_through_release` to reference a concrete release. Canary proves its protocol version and bounded runtime records, but those facts do not prove a datapack-wide reviewed completeness boundary. Reusing the protocol version or inventing a sentinel release would turn missing evidence into a false fact.

Platform persistence also requires a non-null release foreign key, and activation does not need to handle an unknown value because schema `1.0.0` cannot express one.

## Decision

1. Preserve schema `1.0.0` byte-for-byte.
2. Add schema `1.1.0`, changing only `verified_content_through_release` from a release key to a nullable release key.
3. Register schema path and SHA-256 by semantic version in Platform and select the schema from the declared snapshot version.
4. Keep retained schema `1.0.0` snapshots compatible with activation and rollback.
5. Persist a schema `1.1.0` null boundary as a nullable foreign key without a sentinel release.
6. Permit inactive import and inspection of a null-boundary snapshot.
7. Reject activation when the verified boundary is null or earlier than the target profile release.
8. Keep public projection fail closed; a null-boundary snapshot cannot become active.
9. Roll out consumer support before any producer emits schema `1.1.0`.

## Consequences

- Missing completeness evidence is represented honestly.
- Reviewed metadata work can continue without claiming datapack-wide completeness.
- Existing active and rollback snapshots remain usable.
- Older consumers reject schema `1.1.0` fail closed.
- Database rollback to the non-null column is blocked while any null-boundary snapshot exists.
- A separate evidence task is still required before activation.

## Rejected alternatives

- Use protocol `15.25` as the content boundary: protocol compatibility does not prove content completeness.
- Add a fake release such as `0.0.0`: a sentinel would look like evidence and contaminate release ordering.
- Mutate schema `1.0.0` in place: the same version and schema hash would acquire incompatible semantics.
- Treat null as publicly safe: unknown evidence must fail closed.

## Related records

- `docs/architecture/adr/0016-versioned-game-catalog-snapshots.md`
- `docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md`
- Platform task `OTERYN-20260729-game-catalog-null-boundary`
- Canary task `CAN-20260729-game-catalog-schema-1-1`
- Coordination ID `OTS-20260728-game-catalog-v1`
