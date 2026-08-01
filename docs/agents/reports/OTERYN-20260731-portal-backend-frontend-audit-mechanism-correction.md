# Oteryn portal audit — Issue #365 mechanism correction

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## Precedence

This addendum supersedes any inconsistent `DERIVED / HIGH confidence` wording in the earlier consolidated report, evidence index, validator packet or task checkpoint concerning the hypothesis that Playwright's `Publish` action starts an old-document lazy thumbnail request which consumes publication flash before redirect GET.

It does not change the normalized finding totals, defect severity, post-serialization reproduction state or validator verdict.

## Material correction

A new source-faithful Chromium probe copied the actual frozen-source Wiki form ordering and relevant responsive geometry:

- media picker first;
- article settings;
- two translation panels with the real large Markdown-field height;
- categories;
- change-note and save controls;
- Lifecycle and `Publish` last;
- 12 native-lazy media thumbnails;
- exact desktop, tablet and mobile Playwright viewports.

Three immediate-action and three pre-scroll samples were executed per viewport, 18 total.

| Profile | Initially started thumbnails | New starts from `Publish` action start |
|---|---:|---:|
| desktop | 12 | 0 in 6/6 samples |
| tablet | 10 | 0 in 6/6 samples |
| mobile | 4 | 0 in 6/6 samples |

The generic earlier probe placed the control directly below the grid and therefore modeled a materially different scroll path. It remains evidence of generic browser feasibility, not a source-faithful reproduction of Oteryn behavior.

## Corrected classification

```yaml
finding: OTERYN-AUDIT-P35-005
severity: MEDIUM
historical_state: PROVEN
post_serialization_state: REPRODUCED_INTERMITTENT
current_remediation_state: NOT_PROVEN_REMEDIATED
root_cause: UNKNOWN
old_document_lazy_thumbnail_race:
  classification: DERIVED
  confidence: LOW
validator_verdict: VALIDATED_WITH_CORRECTIONS
normalized_totals:
  high: 0
  medium: 6
  low: 1
```

## Preserved facts

- attempt 2 on `6c1e910d...` passed responsive mobile;
- attempts 3 and 4 reproduced the exact mobile transient-status loss;
- durable `Published`, version 3 and `Unpublish to draft` remained present;
- desktop, tablet and portability projects passed;
- desktop/tablet passed despite contaminated thumbnail HTTP 500 traffic;
- both mobile reproductions recorded more completed thumbnail 500 responses and zero aborted requests;
- the existing diagnostic collector lacks request timestamps, initiator identity, `Referer`, correlation ID and session-lock/session-save evidence.

## Remediation correction

Preserving pending `status` specifically across Wiki media-index or thumbnail responses is no longer presented as the smallest proven repair. It is only a candidate that requires exact request/session correlation.

No application, route, view, configuration, migration, committed test, workflow, dependency, deployment or production change was made.

## Remaining gate

The exact frozen 12-sample runbook remains required. Immediate versus pre-scroll remains a useful control but must be interpreted hypothesis-neutrally. `VALIDATED` remains forbidden until the evidence package identifies the request or lifecycle path that removes `status`, or proves the defect absent under clean isolated exact-frozen execution.

Canonical corrected evidence:

- `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.md`;
- `ISSUE_365_SOURCE_FAITHFUL_LAYOUT_PROBE.json`;
- corrected `ISSUE_365_FLASH_REQUEST_LIFECYCLE_ANALYSIS.md`.
