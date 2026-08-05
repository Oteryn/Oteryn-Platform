# Oteryn Platform ADR registry validator

## Identity

```yaml
task: OTERYN-20260805-adr-registry-validator
issue: 577
pull_request: 581
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
repository: blakinio/Oteryn-Platform
exact_base: 3f79987f47e5c7593daccdf1136e09d6641017de
first_failed_head: 2d1d59fffe8d0163ff49a42afb7c0c18d7521655
second_failed_head: b541e7a7c54f73a186cdc8cc2da3491c4acc729f
classification: infrastructure
runtime_change: false
```

## Problem

The accepted ADR authority model preserves historical numeric-prefix collisions but previously relied on manual discipline. A future ADR could add another collision, drift out of the README inventory, omit lifecycle state or point to a missing supersession target without a deterministic repository gate.

## Alternatives

### A — rename historical ADR files

Rejected. Renaming accepted paths would break or obscure inbound references and require a broader compatibility migration.

### B — assign new stable IDs to every ADR

Deferred. Stable IDs may become useful, but mass metadata migration is unnecessary to prevent new defects and would expand this bounded task.

### C — closed exact-path legacy allowlist

Selected. Preserve the eight exact historical duplicate sets and reject any new collision or change to those sets. Filename/path remains the stable identity for this repair.

## Implementation

- `tools/validation/adr_registry.py` uses only the Python standard library.
- `tools/validation/test_adr_registry.py` provides positive, negative and boundary fixtures.
- `tools/validation/phpunit/AdrRegistryValidationTest.php` makes the existing PHPUnit/CI path run the validator and focused suite.
- `phpunit.xml` registers the tooling-owned bridge without editing workflow files.
- No dependency, runtime or production path is changed.

## First exact-head failure — lifecycle syntax compatibility

CI run `31025277136`, PHPUnit job `92372884204`, failed only in `AdrRegistryValidationTest::test_repository_adr_registry_passes`.

The first parser recognized only bullet metadata such as `- Status: Accepted`. Seventeen established ADRs instead used either a plain `Status: Accepted` key or a `## Status` section followed by the lifecycle value.

The parser was repaired to accept exactly one lifecycle declaration in any of the three established forms while still rejecting no declaration or multiple declarations. Ten focused fixtures pass. No historical ADR was rewritten.

Deep System Validation run `31026544499` subsequently passed the complete PHP regression and concurrency suites on `b541e7a7c54f73a186cdc8cc2da3491c4acc729f`, proving the repaired parser against the live registry.

## Second exact-head failure — global native-contract path boundary

Native protocol contract audits run `31026544250` failed only in Audit 1 because its global documentation-boundary check rejects every change under `tests/**`. The other four native protocol audit jobs passed and no protocol invariant failed.

The bridge is therefore moved from `tests/Unit/Architecture/**` to `tools/validation/phpunit/**` and registered in `phpunit.xml`. This preserves automatic execution in the existing `php artisan test` command without changing `.github/workflows/**` or weakening the native audit.

## Enforced invariants

1. Every ADR except `README.md` uses `NNNN-lowercase-hyphenated-slug.md`.
2. Every ADR has exactly one recognized lifecycle declaration.
3. The README inventory equals the directory exactly.
4. A numeric prefix is unique unless its exact path set is one of the eight closed historical exceptions.
5. Changing, adding or removing a path in a historical exception fails closed.
6. A superseded ADR identifies one existing replacement and cannot supersede itself.

## Compatibility

The validator does not alter existing ADR bytes, paths, numeric prefixes or inbound references. It turns the observed historical state into a closed compatibility boundary instead of a reusable exception.

## Validation state

- focused Python fixture suite after parser repair: PASS, 10 tests;
- live repository validator through Deep System PHP regression: PASS;
- Agent Governance on `b541e7a7c54f73a186cdc8cc2da3491c4acc729f`: PASS (`31026544387`);
- Native protocol contract: PASS (`31026544346`);
- Native protocol audit path boundary: root cause proven; bridge relocation pending new exact-head validation;
- exact changed-path audit after relocation: nine bounded tooling/configuration/documentation paths;
- runtime E2E: `NOT_APPLICABLE` because no runtime or user-facing behavior changes.
