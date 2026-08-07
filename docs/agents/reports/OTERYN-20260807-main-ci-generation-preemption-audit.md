# OTERYN-20260807 main CI generation preemption audit

## Finding

`OPA-GOV-0025` — a documentation/governance-only push to `main` can still cancel an in-progress product/runtime-required `CI` push generation because `.github/workflows/ci.yml` groups all `main` pushes into the same concurrency key with `cancel-in-progress: true`.

The later docs-only generation then classifies only its own exact push diff and correctly skips `runtime-tests`; it does not inherit or complete the cancelled product generation's required runtime validation. This is distinct from OPA-GOV-0020 / Issue #783, which repaired docs-only heavy CI execution and docs-only Acceptance emission/preemption but did not prevent the analogous core-CI generation replacement.

## Current-source evidence

- `CI` runs on every push to `main`.
- Its concurrency group is `ci-${{ github.workflow }}-${{ github.ref }}` with `cancel-in-progress: true`, so every main push shares one supersedable generation.
- `scripts/ci/classify_push_changes.py` classifies only the exact event `before..after` range and correctly returns no heavy gates for a valid docs-only diff.
- `runtime-tests` executes only when the current generation's `ci` output is `true`.
- Deterministic routing tests prove docs-only push classification skips heavy gates but do not protect an earlier runtime-required main generation from cancellation by that lightweight generation.

## Live evidence

### Native OAuth security merge followed by lifecycle docs

- Product/security main commit `f6a2b6cefe8ad5993436ac18be8ca4d08919d69b` (`fix(security): bind native OAuth game auth to revocation generation (#825)`) started main CI run `31197719726`.
- That CI run was cancelled at `2026-08-07T16:30:37Z`.
- The immediately following docs-only main commit `8792d3eaefd47b33d27001f1bbe1bd95f0d861d1` (`docs(agents): archive issue #801 repair task (#830)`) created CI run `31197906544` at `2026-08-07T16:30:36Z`.
- The replacement docs-only run passed classification/required-gate jobs while `runtime-tests` was `SKIPPED`.

Therefore the latest docs-only generation can terminate the preceding required product CI generation without performing the runtime tests required by that preceding product diff.

### Independent second occurrence

- Main commit `97c3b24f3d642ac0589efc61e48b66472538aeb9` (`ci(native-protocol): keep producer task routing fail-closed (#834)`) had main CI run `31200041790` cancelled.
- The next lifecycle-only main commit was `3109d5e15e98c9c463130dc736db90667ab83c9a` (`docs(agents): archive issue #829 CI routing task (#836)`).

This independently matches the same shared-main concurrency failure mode.

## Expected invariant

A docs/governance-only main push may run lightweight classification and a required gate, but it must not cancel or replace an earlier main CI generation whose product/runtime diff requires `runtime-tests`. Same-PR supersedable CI cancellation should remain enabled.

## Deduplication

Searches across open and closed Issues for docs-only/main/CI cancellation and runtime preemption found OPA-GOV-0020 / Issue #783 only. #783 explicitly repaired docs-only heavy CI and Acceptance generation/preemption; it did not close this core-CI concurrency root cause. No separate actionable duplicate was found.

## Safety

Audit evidence only. No workflow implementation, runner mutation, production action or external-repository mutation is included here.
