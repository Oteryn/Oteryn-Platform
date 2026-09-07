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

The 13 `one_shot_historical` / `historical_do_not_run` entries remain inert provenance; their bodies are not reinterpreted as current policy. The adoption instruction delta adds only its #1301 task packet. This lifecycle repair archives the already integrated D26 packet without changing D26 code, and its liveness regression remains green. Draft PR #1270 was the only direct stale `AGENTS.md` overlap and was closed without merge after exact successor #1304 was published; #1294, #1298 and #1269 remain separate.

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

The workflow's nested checkout uses the literal reviewed META commit as an independent trust anchor rather than deriving executable code identity from candidate-controlled binding bytes. The authenticated consumer then requires that checkout and the binding to agree, so either value changing alone fails closed.

Focused regressions demonstrate fail-closed behavior at the Platform consumer boundary. Existing task, liveness, Documentation IA and prompt suites exercise representative local consumers. That deterministic delivery evidence alone does not establish model adherence; W5 therefore evaluated repeated task behavior separately.

W5 subsequently completed bounded matched model-task screening against admission `3557085c20512d25576d8884cc54471665784b00` and the immutable reviewed instruction tree `3934259316b65b251a82710284fa02e15859a62b` (local `bad2e5a2bcf5193ecb6418a21526d062e49115bc`, remote equivalent `48c19ad5c74ca933e057e705ade7b5b4a1acec12`). Four fresh `gpt-5.6-sol` medium threads accepted 16/16 case executions: five unique cases per baseline/candidate arm plus three safety repeats per arm. Both arms rejected the out-of-scope domain request, refused a hostile cached-authority grant, preserved same-session continuation without a background-completion claim, produced the expected typo-only code fixture, passed 18 independently rerun identifier assertions and used the representative remediation prompt. The canonical result is PR #1304 comment `5568582769`; local comparison JSON SHA-256 is `6f4a37ba06d7f0f6d13ca552a9c6c72ff6e726935ace9698e8577ea35b71f0da`.

This finite synthetic screen supports bounded behavioral parity for the sampled tasks, not production behavior, universal prompt delivery, or statistical efficiency. All path/read/tool/import/cleanup failures remain in the run record. Token, cache, reasoning and billing telemetry were unavailable, so no token, money or productivity saving is claimed.

Product runtime E2E is `NOT_APPLICABLE`: no Laravel service, authentication/session, data/persistence, payment, deployment artifact or production configuration changes.

### Hosted lifecycle and W6 observations

The first #1304 run proved the online authenticated consumer and its 13 regressions, then correctly failed repository lifecycle checks because the task packet omitted its newly created PR and the already merged D26 packet remained active. This repair records #1304 and archives D26 using PR #1300, protected-main commit `3557085c20512d25576d8884cc54471665784b00`, closed Issue #1299 and source-ref absence. CodeQL also identified the candidate-derived nested-checkout ref; the independent literal trust anchor above repairs that trust boundary without changing any W5-tested prompt or instruction blob.

The initial PR observation covered 270 executed-job-seconds as a duration proxy, including the failed governance/branch-hygiene jobs and 196 seconds for CodeQL. Product-heavy runtime probes were skipped by the existing impact classifier, while governance, branch hygiene, CodeQL and the repository's other selected gates remained active. Final exact-head PR, merge-queue and protected-main durations remain pending; this is delivery evidence, not a token or cost-savings claim.

Historical Branch Audit run `34104877215`, job `101687493437`, artifact `10011942363` separately reported two unexplained remote refs. Both are validation-only canaries from closed, unmerged PRs (#1290 at `d866d21ca856b925a32a3a5457174906237db199`, #1291 at `65126f2b1d24e0fe497e436be99f524104021033`) whose bodies forbid merge; a fresh 2026-09-07 branch search confirms both named refs still exist. The active #1301 task records narrow interim `lock_branch` claims with exact heads and recovery evidence. Although direct connector deletion and authenticated Git publication are unavailable here, the existing protected-main `terminal-branch-lifecycle` workflow can apply a separately reviewed canonical terminal-candidate digest with live exact-SHA, PR, claim, protection, retention, policy and digest revalidation. The cleanup therefore waits for a separate normal PR that removes the interim claims, captures its live dry-run manifest, binds the resulting approval and passes review. The refs remain pending deletion and absence readback; this is not a deletion claim, a validation exception or part of the policy-adoption qualification, and no controller change is proposed to force eligibility.

## Rollback

Revert the coherent adoption commit to restore the prior local instructions, prompts, standards, validators and workflow. Do not leave workflow/binding halves at incompatible revisions. Product runtime and persistent data need no migration or rollback.

## W6 opportunities

1. Use the real #1301 PR, Merge Queue candidate and protected-main run as the governance canary; inspect the existing #1268 protocol rather than creating a synthetic no-op PR.
2. Review the pre-existing #1012 checkpoint-only heavy-workflow concern separately. Current classification excludes agent-governance-only changes from product runtime workflows, so this adoption does not broaden runtime CI.
3. Extend the completed bounded W5 model-task screen with broader task sampling and ablation before any further consolidation of routed continuation/closeout documents.
4. Consider moving the authenticated authority/tree/blob primitive into META as a reusable library after provider adoption proves the interface; avoid copying it among providers.
5. Measure actual delivery and repeated task outcomes before any organization-wide efficiency conclusion. Source-byte reduction alone supports no token or cost claim.
