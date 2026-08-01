# Issue #365 embedded browser diagnostics

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen audit target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Direct diagnostic source: `6c1e910d36771f50da5eded93cc50274a90c62d2`

## Purpose

Preserve the complete sanitized `browser-diagnostics` attachments for the two post-session-serialization attempts that reproduced the original responsive-mobile publication-flash loss.

The HTML reports embed a base64-encoded Playwright report ZIP. Decoding that embedded report recovers diagnostics for successful desktop/tablet tests as well as the failed mobile test. This evidence was not inferred from JUnit totals or screenshots.

## Artifact identity

| Workflow attempt | Job | Artifact | GitHub/download SHA-256 | HTML report SHA-256 | Embedded `report.json` SHA-256 |
|---:|---:|---:|---|---|---|
| 3 | `91343023604` | `8815383351` | `7498934d30f5292dab91e46edbc5659bc885acc11fa84c1784cb2525d8cd48a8` | `027caae8b0ef42cce6a666dc0fd72d2d59abe3919e5b77197c55d9d932a3610b` | `8e9b5a7685d2ec984b80c7d9242bedbf687ece8820380d05154648dceba7490f` |
| 4 | `91343514611` | `8815457044` | `790bc6cc4a7777b591abca9575cdb6927fb7c93f2682694f09e03285131d2bba` | `715b9556898c2206dd79a68732cccc7818376ac0f153c99335aca9a1a65cdc32` | `af0c99da60e4ad54d07b8d3d645f1e2df87a4740ba5d594aa54b23f9484105e7` |

Both attempts belong to workflow run `30612399525`, use exact tested SHA `6c1e910d36771f50da5eded93cc50274a90c62d2`, real Laravel HTTP and Playwright retries `0`.

## Attempt 3 diagnostics

| Project | Original flow | Duration | Thumbnail HTTP 500 | Media-ID distribution | Failed requests | Invalid-pattern console errors | Page errors |
|---|---|---:|---:|---|---:|---:|---:|
| `responsive-desktop` | PASS | 3,165 ms | 9 | `1×3, 3×3, 5×3` | 6 | 2 | 0 |
| `responsive-tablet` | PASS | 3,162 ms | 12 | `1×3, 3×3, 5×3, 7×3` | 8 | 2 | 0 |
| `responsive-mobile` | REPRODUCED | 12,879 ms | 16 | `1×3, 3×2, 5×4, 7×4, 9×3` | 0 | 2 | 0 |

The mobile failure is the exact missing `role=status` assertion. Its error context independently shows durable `Published`, version 3 and `Unpublish to draft`.

## Attempt 4 diagnostics

| Project | Original flow | Duration | Thumbnail HTTP 500 | Media-ID distribution | Failed requests | Invalid-pattern console errors | Page errors |
|---|---|---:|---:|---|---:|---:|---:|
| `responsive-desktop` | PASS | 3,193 ms | 9 | `1×3, 3×3, 5×3` | 6 | 2 | 0 |
| `responsive-tablet` | PASS | 4,544 ms | 12 | `1×3, 3×3, 5×3, 7×3` | 8 | 2 | 0 |
| `responsive-mobile` | REPRODUCED | 13,014 ms | 14 | `1×3, 3×2, 5×3, 7×3, 9×3` | 0 | 2 | 0 |

The mobile failure again lacks the transient status while preserving durable `Published`, version 3 and `Unpublish to draft`.

## Proven boundaries

`PROVEN`:

- the same deterministic stale-media ID expansion is present after session serialization: desktop `1/3/5`, tablet `1/3/5/7`, mobile `1/3/5/7/9`;
- desktop and tablet retain publication feedback despite 9 and 12 thumbnail HTTP 500 responses;
- mobile loses publication feedback in both preserved reproductions while observing the expanded stale-ID set;
- the invalid HTML pattern emits exactly two console errors per original-flow project;
- neither mobile reproduction records a Playwright `failedRequests` entry or page error even though response diagnostics contain 14 or 16 HTTP 500 responses.

`NOT PROVEN`:

- thumbnail integrity failures cause the mobile flash loss;
- the difference between 14 and 16 mobile responses changes the outcome;
- valid media fails in a clean isolated sample;
- session serialization is ineffective for every concurrency condition;
- exact frozen-target clean behavior.

Desktop/tablet success under contaminated state disproves any simple rule that the presence of thumbnail HTTP 500 responses alone necessarily removes the flash. The evidence still supports a shared timing-sensitive environment, but no causal relationship may be asserted without the clean-versus-exactly-one-damaged-row comparison.

## Audit impact

The diagnostics strengthen, but do not change, the normalized finding set:

- `OTERYN-AUDIT-P35-006` remains `MEDIUM`: acceptance fixture isolation/evidence failure;
- `OTERYN-AUDIT-P35-005` remains `MEDIUM`: post-serialization original mobile flow is `REPRODUCED_INTERMITTENT` and `NOT_PROVEN_REMEDIATED`;
- `OTERYN-AUDIT-P35-007` remains `MEDIUM`: two deterministic native-pattern console errors per original-flow project.

The exact frozen clean-isolated and exactly-one-damaged-row package remains required. No application, test, workflow, deployment or Canary mutation was performed.