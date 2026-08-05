# Oteryn Platform ADR registry validator

## Identity

```yaml
task: OTERYN-20260805-adr-registry-validator
issue: 577
pull_request: 581
repository: blakinio/Oteryn-Platform
implementation_head: b2de0c6cd63a9313e8116204893ba2c0a1d9db6d
implementation_merge: 2a9715f89a38d2e8e441d34813f03bc0ad6dd707
status: completed
runtime_change: false
```

## Final result

The ADR registry is now machine-validated without renaming or renumbering accepted history.

The validator enforces:

1. `NNNN-lowercase-hyphenated-slug.md` filenames;
2. exactly one recognized lifecycle declaration in any established repository form;
3. exact equality between the ADR directory and README inventory;
4. unique numeric prefixes except for eight closed exact-path legacy sets;
5. fail-closed detection of any change to those legacy sets;
6. resolvable, non-self-referencing supersession targets.

Ten focused fixtures cover the accepted, negative and boundary cases. The normal PHPUnit suite executes the Python validator through a tooling-owned bridge registered in `phpunit.xml`. No workflow file or dependency changed.

## Compatibility decision

The selected model is a closed exact-path legacy allowlist. Filename/path remains the stable inbound-reference identity. New duplicates and changes to historical duplicate sets are rejected. Mass renaming, aliases and per-ADR stable-ID migration remain unnecessary for this bounded repair.

## Repaired validation findings

### Lifecycle syntax compatibility

The first implementation recognized only bullet `- Status:` metadata. CI run `31025277136`, job `92372884204`, artifact `8938486455` proved that seventeen accepted ADRs also use plain `Status:` or `## Status` declarations. The parser was expanded to the three established forms while preserving the exactly-one rule. No historical ADR was rewritten.

### Native-contract path boundary

A prior PHPUnit bridge under `tests/**` caused native protocol audit run `31026544250` to fail its global path boundary. The bridge moved to `tools/validation/phpunit/**` and was registered in `phpunit.xml`. Native protocol contract audits then passed without policy weakening.

### Checkpoint schema

Agent Governance run `31027624871` rejected an unsupported nested checkpoint key. The checkpoint was corrected, and exact final-head Agent Governance passed.

## Exact final validation

Head `b2de0c6cd63a9313e8116204893ba2c0a1d9db6d`:

- CI `31028447057`: PASS;
- Agent Governance `31028446915`: PASS;
- Phase 7 `31028446874`: PASS;
- Edge Security `31028446882`: PASS;
- Game Auth `31028446947`: PASS;
- DB Outage `31028446904`: PASS;
- Native protocol contract `31028446949`: PASS;
- Native protocol contract audits `31028446889`: PASS;
- changed-path and ownership audit: PASS;
- review threads: 0;
- runtime E2E: `NOT_APPLICABLE`.

## Scope proof

The merged PR changed exactly nine bounded tooling, configuration, architecture-documentation and task-record paths. It changed no ADR file except the registry README and changed no runtime, migration, workflow, dependency, deployment, production, external-repository, PR #541, PR #542 or Issue #558-owned path.

## Next bounded domain

`ARCH-AUTH-004`: reconcile current system and module architecture from PR #453 and later exact merged evidence.
