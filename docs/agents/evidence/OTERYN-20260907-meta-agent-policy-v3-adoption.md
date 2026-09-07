# Platform META policy v3 adoption evidence

## Identity

- Platform admission: `3557085c20512d25576d8884cc54471665784b00`
- Platform Issue: #1301
- META authority commit: `5ed3f14400af450b5875c091e443da70f2d67ab9`
- META policy: `OTERYN_ORGANIZATION_AGENT_POLICY@3.0.0`

The binding uses the first protected-main revision in this lane that includes the v3 policy plus the corrected routing, continuation-capability and access/late-integration machine authorities. GitHub identity checks authenticate that exact commit, its protected-main ancestry and every locally loaded authority blob before executing central code.

## Source volume

Counts use UTF-8 repository bytes and newline counts from Platform admission and the candidate worktree. They describe source volume only and are not token, cost or productivity estimates.

| Surface | Files | Admission lines / bytes | Candidate lines / bytes | Change |
|---|---:|---:|---:|---:|
| Root, Platform bootstrap and nested agent instructions | 3 | 578 / 55,134 | 135 / 11,192 | -443 / -43,942 |
| Local prompting and prompt-eval standards | 2 | 360 / 19,793 | 45 / 3,836 | -315 / -15,957 |
| Local policy validator and regression tests | 2 | 1,217 / 69,393 | 552 / 25,343 | -665 / -44,050 |
| Active reusable prompt bodies | 10 | 2,494 / 137,233 | 664 / 44,661 | -1,830 / -92,572 |
| Three prompt text suites | 3 | 628 / 26,400 | 548 / 21,797 | -80 / -4,603 |

## W4 prompt and task inventory

`DOCUMENTATION_IA_CATALOG.json` remains the executable/inert authority: 23 prompts, comprising ten active reusable prompts and 13 historical non-executable prompts. All ten active prompts were migrated semantically, not merely accepted because an earlier lint passed.

| Active prompt | Preserved Platform delta |
|---|---|
| Remediation | one finding owner, risk gate, complete repair and terminal lifecycle |
| Game Catalog | #301/#338 consumer hold, producer provenance, fail-closed import and public/admin delivery |
| Character Lifecycle | independent #317/#319/#320 operations, native authority unknowns and Bazaar conflicts |
| Continuous Audit | non-implementation audit authority, coverage domains, finding schema and remediation handoff |
| Platform Wave | reserved sibling ownership, live wave barriers and selector rerun |
| Portal Completion | sole-selector ordering, non-scheduling scope, architecture routing and full-stack acceptance |
| Migration Ultra | delta-first frontier, blocker classes and transaction subordination |
| Architecture Review | decision classification, alternatives, proposed/accepted boundary and implementation handoff |
| Migration Programme | authenticated coordinates, caller/provenance inventory, transaction states, rollback/replay and cutover verification |
| Portal Parallel Coordinator | independence gate, durable worker package and dependency-safe integration |

The obsolete `blakinio/Oteryn-Platform` owner coordinate and copied organization execution/continuation/communication/model controllers were removed from every active prompt. Five active programme/allocation sources referenced by those prompts were also reconciled to `Oteryn/Oteryn-Platform` and current bound service/model authority so delegated live selection cannot fall back to stale ownership inputs. Deterministic prompt suites now require stable named task invariants plus independently meaningful semantic clauses. Domain constraints remain in prose and canonical programme/architecture references. Static evaluation does not establish model adherence.

The 13 `one_shot_historical` / `historical_do_not_run` entries remain inert provenance; their bodies are not reinterpreted as current policy. This task adds only its #1301 task packet. D26 remains untouched and covered by its liveness regression. Draft PR #1270 is the only direct stale `AGENTS.md` overlap and is reconciled only after a successor candidate exists; #1294, #1298 and #1269 remain separate.

## Validation layers

### Contract and authenticated loading

- Binding is closed to the expected policy ID/version/repository/path set before any central code is loaded.
- The META checkout must have the exact bound HEAD, a matching origin and a clean tracked/untracked worktree.
- GitHub commit, protected `main`, compare/merge-base and recursive tree responses prove protected-main ancestry and exact Git blob identity.
- The loader rechecks the central module digest and clean checkout immediately before import, then invokes central bundle, binding, Platform overlay and active prompt validation.
- Focused consumer regressions: 13/13 PASS, including dirty executable checkout and authenticated blob-mismatch rejection.
- Prompt suites: main 24 cases/11 categories/9 safety-critical; Platform wave 11/11/4; Portal parallel 11/11/3. All PASS with `model_trials_executed=0`.

A local unauthenticated online rerun reached GitHub’s public API rate limit. That is recorded as environment evidence, not a policy PASS; exact-head Actions supplies `github.token` to the same validator. Independent code review verified the exact 5ed3 checkout, loader ordering and focused tests; protected-main GitHub authentication remains an exact-head Actions gate.

### Delivery and representative behavior

Root startup resolves `META_AGENT_POLICY_BINDING.json`; the workflow checks out the bound META revision and exposes no direct pre-authentication execution path. Platform checks run with bytecode disabled so validation cannot dirty the executable authority checkout. The central consumer is the only code-loading path.

Focused regressions demonstrate fail-closed behavior at the Platform consumer boundary. Existing task, liveness, Documentation IA and prompt suites exercise representative local consumers. This is deterministic delivery evidence. Level-3 model behavior is `NOT_EVALUATED`; actual repeated task trials remain a separate W5 activity.

Product runtime E2E is `NOT_APPLICABLE`: no Laravel service, authentication/session, data/persistence, payment, deployment artifact or production configuration changes.

## Rollback

Revert the coherent adoption commit to restore the prior local instructions, prompts, standards, validators and workflow. Do not leave workflow/binding halves at incompatible revisions. Product runtime and persistent data need no migration or rollback.

## W6 opportunities

1. Use the real #1301 PR, Merge Queue candidate and protected-main run as the governance canary; inspect the existing #1268 protocol rather than creating a synthetic no-op PR.
2. Review the pre-existing #1012 checkpoint-only heavy-workflow concern separately. Current classification excludes agent-governance-only changes from product runtime workflows, so this adoption does not broaden runtime CI.
3. Run representative model task trials and ablation before any further consolidation of routed continuation/closeout documents.
4. Consider moving the authenticated authority/tree/blob primitive into META as a reusable library after provider adoption proves the interface; avoid copying it among providers.
5. Measure actual delivery and repeated task outcomes before any organization-wide efficiency conclusion. Source-byte reduction alone supports no token or cost claim.
