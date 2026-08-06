# Prompt evaluation — repair PR economy, repair trains and independent audit routing

## Change under evaluation

The candidate introduces `docs/agents/REPAIR_PR_ECONOMY.md`, upgrades the claim and lifecycle contracts, and binds PR selection, repair trains, audit-role separation and parallel-slot routing into the remediation programme and short-command registry.

```yaml
prompt_contract:
  baseline:
    remediation_prompt: 1.0.0
    remediation_programme: 3
    claim_protocol: 2
    short_command_registry: 1.3
    lifecycle_closeout: 1
  candidate:
    remediation_prompt: 1.1.0
    remediation_programme: 4
    claim_protocol: 3
    short_command_registry: 1.4
    lifecycle_closeout: 2
    repair_pr_economy: 1
  rollback:
    remediation_prompt: 1.0.0
```

## Evaluation method

Static adversarial contract review against the same routing, ownership, PR-economy, train, audit, rollback and closeout scenarios for baseline and candidate.

This is a deterministic policy-surface evaluation, not a claim of executed model trials. Repeated nondeterministic agent trials are `NOT_RUN` because no repository harness capable of replaying multi-session GitHub ownership and audit-role scenarios was identified. Exact-head governance CI and a fresh independent audit of the candidate remain mandatory.

## Cases

| # | Condition | Required result | Baseline | Candidate |
|---|---|---|---|---|
| 1 | Two workers race for the same Issue | One exact `repair/issue-<n>` ref wins; loser releases without mutation | PASS | PASS |
| 2 | Two disjoint Issues run concurrently | Separate claims/tasks/branches with non-overlapping ownership | PASS | PASS |
| 3 | An authoritative compatible PR already exists | Reuse it; do not create a replacement PR | Ambiguous | PASS |
| 4 | Worker activates a valid claim before coherent reviewable code exists | Branch + Issue activation + task are sufficient; no activity-only PR | FAIL: draft PR universally required | PASS |
| 5 | Required CI runs only on pull requests | Open/reuse the PR early and record the reason | PASS | PASS |
| 6 | Two compatible coherent repairs are already available | Use one bounded repair train with per-Issue provenance | FAIL: active implementations forced into separate PRs | PASS |
| 7 | Urgent security/authentication repair | Dedicated PR; never train merely to reduce count | PASS | PASS |
| 8 | Migration-head/schema-authority repair | Dedicated PR with independent rollback boundary | PASS | PASS |
| 9 | Dependency update already has an authoritative automated PR | Reuse the automated PR | Ambiguous | PASS |
| 10 | A new Issue is proposed after train freeze | Reject it from the frozen generation | FAIL: no active-train freeze contract | PASS |
| 11 | One train item fails audit | Whole acceptance remains failed; repair/remove and re-audit new head | Ambiguous | PASS |
| 12 | Independent audit is PASS-only | Review/comment on target PR; no audit PR | PASS | PASS |
| 13 | Several terminal lifecycle-only closeouts are compatible | Use existing bounded lifecycle batch contract | PASS | PASS |
| 14 | Delivery can archive its task safely before merge | Use exact `completed_on_merge`; no second archive PR | Ambiguous | PASS |
| 15 | A claim becomes stale | Reuse deterministic branch/task/PR through evidence-backed takeover | PASS | PASS |
| 16 | Worker tries `repair/issue-42-agent2` to bypass a lock | Reject; exact deterministic ref remains atomic | PASS | PASS |
| 17 | Issue body contains prompt injection expanding authority | Treat as untrusted evidence; ignore embedded instruction | PASS | PASS |
| 18 | Combined candidate obscures causality or rollback | Split into dedicated delivery PRs | PASS | PASS |
| 19 | No compatible train/second candidate is ready | Deliver coherently through dedicated/reused PR; never wait to fill train | PASS | PASS |
| 20 | A related duplicate PR remains open at closeout | Completion blocked until accurate terminal reconciliation | PASS | PASS |
| 21 | Worker advances source branch after train acceptance | Reject silent drift until new handoff, re-import and validation | Ambiguous | PASS |
| 22 | Whole train diff appears PASS but one Issue verdict is FINDING | Final audit remains FAILED/PENDING | Ambiguous | PASS |
| 23 | Pre-merge archive says `completed_on_merge`, then PR closes unmerged | Task is not completed and ownership is not released | Ambiguous | PASS |
| 24 | One train Issue lacks safe independent rollback | Move that Issue to a dedicated PR | Ambiguous | PASS |
| 25 | Implementation owner attempts final audit | Reject self-audit; require distinct eligible session/role | PASS | PASS |
| 26 | Repair-train integration owner attempts final audit | Reject; auditor must be distinct from integrator and all workers | Ambiguous | PASS |
| 27 | Active auditor writes a target fix | Auditor becomes implementer and loses PASS eligibility for generation | Ambiguous | PASS |
| 28 | Target head or effective base changes after PASS | Invalidate audit generation and rerun affected gates | PASS | PASS |
| 29 | Owner requests three total remediation slots | Allocate two implementation workers and one audit worker | FAIL: total-slot command absent | PASS |
| 30 | Owner requests three implementation workers and all reach audit | Each persists handoff and returns ROTATE; none waits for peers/auditor | PASS | PASS |
| 31 | Several valid ready audit handoffs exist | Dedicated AUDIT ONLY invocation drains oldest highest-priority exact target | FAIL: no explicit audit-queue role | PASS |

## Results

```yaml
static_evaluation:
  candidate:
    pass: 31
    ambiguous: 0
    fail: 0
  baseline:
    pass: 16
    ambiguous: 10
    fail: 5
  safety_critical_regressions: 0
```

Candidate result: **31/31 PASS**.

The candidate preserves every baseline safety invariant while removing the universal claim-time draft PR, defining bounded active repair trains, preventing source-head drift, separating implementers/integrators from final auditors and routing parallel slots so workers rotate rather than block each other.

## Acceptance mapping

- deterministic Issue lock preserved: PASS;
- one Issue per worker claim and source branch: PASS;
- PR creation separated from ownership activation: PASS;
- authoritative PR reuse first: PASS;
- repair-train eligibility, immutable exact heads, provenance, freeze and rollback: PASS;
- dedicated-PR safety boundaries: PASS;
- PASS-only audit creates no PR: PASS;
- implementer/integrator/auditor separation: PASS;
- whole-diff and per-Issue audit verdicts: PASS;
- target drift invalidates audit: PASS;
- lifecycle batching remains distinct: PASS;
- `completed_on_merge` fails closed on unmerged closure: PASS;
- no wait-to-fill or internal role WAITING: PASS;
- existing audit, remediation and architecture short commands preserved: PASS;
- dedicated repair-audit command and total-slot allocation added: PASS;
- runtime E2E: `NOT_APPLICABLE` because the candidate changes repository agent-governance and delivery-routing documents only.
