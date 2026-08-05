# Oteryn Platform ADR registry validator

## Identity

```yaml
task: OTERYN-20260805-adr-registry-validator
issue: 577
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
repository: blakinio/Oteryn-Platform
exact_base: 3f79987f47e5c7593daccdf1136e09d6641017de
classification: infrastructure
runtime_change: false
```

## Problem

The accepted ADR authority model preserves historical numeric-prefix collisions but previously relied on manual discipline. That allowed a future ADR to add another collision, drift out of the README inventory, omit lifecycle state or point to a missing supersession target without a deterministic repository gate.

## Alternatives

### A — rename historical ADR files

Rejected. Renaming accepted paths would break or obscure inbound references and would require a broader compatibility migration.

### B — assign new stable IDs to every ADR

Deferred. Stable IDs may become useful, but mass metadata migration is unnecessary to prevent new defects and would expand this bounded task.

### C — closed exact-path legacy allowlist

Selected. Preserve the eight exact historical duplicate sets and reject any new collision or change to those sets. Filename/path remains the stable identity for this repair.

## Implementation

- `tools/validation/adr_registry.py` uses only the Python standard library.
- `tools/validation/test_adr_registry.py` provides positive, negative and boundary fixtures.
- `tests/Unit/Architecture/AdrRegistryValidationTest.php` makes the existing PHPUnit/CI path run the validator and its focused suite.
- No workflow file, dependency or runtime path is changed.

## Enforced invariants

1. Every ADR except `README.md` uses `NNNN-lowercase-hyphenated-slug.md`.
2. Every ADR has exactly one recognized lifecycle line.
3. The README inventory equals the directory exactly.
4. A numeric prefix is unique unless its exact path set is one of the eight closed historical exceptions.
5. Changing, adding or removing a path in a historical exception fails closed.
6. A superseded ADR identifies one existing replacement and cannot supersede itself.

## Compatibility

The validator does not alter existing ADR bytes, paths, numeric prefixes or inbound references. It turns the observed historical state into a closed compatibility boundary instead of a reusable exception.

## Validation plan

- focused Python fixture suite;
- validator against the live repository tree;
- PHPUnit bridge test;
- exact changed-path audit;
- exact-head GitHub Actions;
- fresh post-implementation diff and invariant audit;
- runtime E2E: `NOT_APPLICABLE` because no runtime or user-facing behavior changes.
