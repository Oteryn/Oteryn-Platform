# GitHub Actions storage hygiene live evidence

Sanitized live cleanup evidence for task `OTERYN-20260811-github-actions-storage-hygiene`.

## Delivery identity

- implementation PR: #980
- implementation head: `8f6efc6b08f9c7b31160db5ec1ffdb032c83e9c2`
- merge commit: `11e223f5f0883f0f3096769fbc2291de7edae62e`
- workflow run: `31476011425`
- successful maintenance jobs:
  - attempt 1: `93729781096`
  - attempt 2: `93733099349`
- mode: `cleanup`
- retention: artifacts 14 days; completed workflow runs 30 days; closed-PR merge-ref caches immediately
- per-run delete budget: 700 exact resources

## Attempt 1

Pre-cleanup:

- artifacts: 15,525 / 12,224,098,506 bytes
- active caches: 1,127 / 5,498,901,543 bytes
- eligible closed-PR caches: 890 / 2,299,473,709 bytes
- eligible old artifacts: 4,819 / 4,592,254,135 bytes
- eligible old completed runs: 0

Deleted:

- artifacts: 494 / 4,244,404,516 bytes
- closed-PR caches: 206 / 2,273,009,806 bytes
- workflow runs: 0

Post-cleanup:

- artifacts: 15,042
- active caches: 929 / 3,248,921,261 bytes
- remaining eligible candidates due to the per-run safety budget: 5,009
- primary rate remaining: 4,117

## Attempt 2

Pre-cleanup:

- artifacts: 15,045 / 7,984,212,306 bytes
- active caches: 915 / 3,225,735,706 bytes
- eligible closed-PR caches: 678 / 26,307,872 bytes
- eligible old artifacts: 4,366 / 358,949,677 bytes
- eligible old completed runs: 0

Deleted:

- artifacts: 531 / 322,357,146 bytes
- closed-PR caches: 169 / 22,419,524 bytes
- workflow runs: 0

Post-cleanup:

- artifacts: 14,523
- active caches: 906 / 3,224,537,526 bytes
- remaining eligible candidates due to the per-run safety budget: 4,344
- primary rate remaining: 3,248

## Combined bounded live result

- exact resources deleted: 1,400
- artifacts deleted: 1,025 / 4,566,761,662 bytes
- closed-PR caches deleted: 375 / 2,295,429,330 bytes
- workflow runs deleted: 0 (repository age was below the 30-day run-retention threshold)
- combined explicit reclaimed bytes: 6,862,190,992 bytes (~6.39 GiB)

The count snapshots can move between attempts because normal CI continues creating new recent artifacts/caches. Cleanup predicates never target recent evidence, open-PR caches, default/branch caches, packages, GHCR images, releases, repository content, environments, secrets or unrelated resources.

The remaining historical backlog is intentionally bounded by the 700-delete safety budget. Permanent `pull_request_target` cleanup prevents new closed-PR cache accumulation, and retained scheduled/manual maintenance drains qualifying historical resources in bounded passes without broad deletion or rate-limit bypass.
