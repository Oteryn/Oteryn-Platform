# Oteryn Platform Remediation Programme

```yaml
prompt_contract:
  version: 2.0.0
  programme_id: OTERYN_PLATFORM_REMEDIATION
  objective: Close one confirmed Platform finding through one accountable implementation owner, complete delivery, risk-proportional evidence and terminal closeout.
  baseline_version: 1.3.0
  rollback_version: 1.3.0
  required_invariants:
    - one_issue_one_owner
    - risk_based_independent_review
    - complete_vertical_slice
    - terminal_lifecycle
owner_alias: OTERYN_PLATFORM_REMEDIATION
```

## Outcome and ownership

Own one eligible unclaimed repair Issue in `Oteryn/Oteryn-Platform` from root-cause proof through the delivered fix and terminal lifecycle. Acquire its deterministic claim branch before changing Issue labels or declaring ownership. Findings from self-review, CI or review return to the same implementation owner.

Do not create a second repair PR, idle approval lane, audit Issue or repair train for the same fix. Apply the independent review selected by the bound META AI-review policy; advisory review does not transfer implementation ownership or replace Platform gates.

## Selection and scope

Resolve live remediation programme state, the claim protocol, Issue taxonomy, PR economy, risk gate and current ownership. Select one implementation-authorized Issue with complete acceptance and no unresolved blocker. Record its exact owned paths in an active task packet.

Keep changes within that Issue. Preserve Platform security, data ownership, compatibility and rollback boundaries. Production, protected-environment, credential, live-auth/session, payment and external-repository operations require separate exact authority.

## Risk and acceptance delta

Classify the repair through `docs/agents/REMEDIATION_AUDIT_RISK_GATE.md`:

- `STANDARD`: bounded reversible low/medium repair with deterministic evidence;
- `HEIGHTENED`: critical/high, security, payment, integrity, concurrency, migration, public contract, dependency, CI/deployment, production, architecture or cross-repository boundary;
- `BLOCKED`: unresolved authority, rollback, compatibility, environment, ownership, `UNKNOWN` or `CONFLICT`.

Prove root cause, implement the smallest complete applicable slice, and add focused regression coverage for the repaired behavior. Heightened work adds the negative-path, rollback, compatibility and final validation evidence required by its risk; it still follows the bound independent-review policy.

Before readiness, record exact-candidate full-diff self-review, acceptance, negative paths where applicable, rollback, compatibility, related PRs and remaining findings. Product-facing repairs require the applicable real E2E path. Resolve all material findings, requested changes and review threads before integration.

## Parallel request delta

A request for multiple remediation workers means distinct Issue owners with non-overlapping branches and paths. A losing claim selects another eligible Issue. Do not reserve a worker solely as a standing approver.

## Stop delta

Stop for an unresolved authority, ownership, safety, product, rollback or compatibility decision. Persist the precise blocker and one next action. A separate continuous-audit programme may discover future findings; it does not take ownership of this repair.
