from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


system_path = Path("docs/architecture/SYSTEM_ARCHITECTURE.md")
gateway_path = Path("docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md")
task_path = Path("docs/agents/tasks/active/OTERYN-20260806-game-auth-topology-reconciliation.md")

system = system_path.read_text(encoding="utf-8")
gateway = gateway_path.read_text(encoding="utf-8")
task = task_path.read_text(encoding="utf-8")

system = replace_once(
    system,
    "Native Game Session contract version 2 is a separate disabled-by-default rollout. Active producer PR #542 does not prove an Otheryn/OTClient consumer, cross-repository cutover or production activation. `PRODUCTION_PROVEN=false` until exact deployed revisions, edge/private ingress, service authentication/rotation and legacy-path disposition are verified together.",
    "Native Game Session contract version 2 is a separate disabled-by-default rollout. The producer package associated with PR #542 does not by itself prove an Otheryn/OTClient consumer, cross-repository cutover or production activation. `PRODUCTION_PROVEN=false` until exact deployed revisions, edge/private ingress, service authentication/rotation and legacy-path disposition are verified together.",
    "system architecture native-v2 evidence wording",
)

gateway = replace_once(
    gateway,
    "The selected native Game Session contract version 2 remains disabled by default and separately governed. Active PR #542 is producer-only work and is not evidence that a native consumer or production cutover is complete.",
    "The selected native Game Session contract version 2 remains disabled by default and separately governed. The producer package associated with PR #542 is not, by itself, evidence that a native consumer, cross-repository cutover or production activation is complete.",
    "gateway contract native-v2 evidence wording",
)

task = replace_once(task, "updated_at: 2026-08-06T15:12:00+02:00", "updated_at: 2026-08-06T15:55:00+02:00", "checkpoint updated_at")
progress_marker = "last_progress_at: 2026-08-06T15:12:00+02:00"
if task.count(progress_marker) != 2:
    raise SystemExit(f"checkpoint/recovery last_progress_at: expected exactly two matches, found {task.count(progress_marker)}")
task = task.replace(progress_marker, "last_progress_at: 2026-08-06T15:55:00+02:00", 1)
task = replace_once(
    task,
    """status: validating
phase: audit_remediation_validate
session_id: chatgpt-20260806T1512+0200-game-auth-topology-audit-remediation
session_role: implementer
execution_mode: github
execution_reason: independent audit review 4874934896 identified one high-severity internal contradiction; apply the smallest documentation-only repair and return the new exact head to CI and a different fresh validator""",
    """status: validating
phase: audit02_remediation_validate
session_id: chatgpt-20260806T1555+0200-game-auth-topology-audit02-remediation
session_role: implementer
execution_mode: github
execution_reason: independent re-audit Issue #750 / review 4875167020 preserved medium finding OPA-ARCH-20260806-001-AUDIT-02; replace transient PR-liveness wording with status-independent evidence wording and return the successor exact head to CI and a different fresh validator""",
    "checkpoint execution state",
)
task = replace_once(task, "ci_check_generation: final_audit_target", "ci_check_generation: audit02_remediation", "CI generation")
task = replace_once(task, "repair_cycles_for_current_gate: 2", "repair_cycles_for_current_gate: 3", "repair cycle counter")
task = replace_once(
    task,
    "  - Independent audit Issue #737 and PR review 4874934896 recorded high finding OPA-AUD-731-001: the current overlay was contradicted by stale later wording that said Platform was not in the game-authentication path.",
    "  - Independent audit Issue #737 and PR review 4874934896 recorded high finding OPA-AUD-731-001: the current overlay was contradicted by stale later wording that said Platform was not in the game-authentication path.\n  - Independent re-audit Issue #750 and PR review 4875167020 confirmed OPA-AUD-731-001 resolved and preserved medium finding OPA-ARCH-20260806-001-AUDIT-02: durable canonical documents encoded transient `Active PR #542` liveness wording.",
    "audit02 finding evidence",
)
task = replace_once(
    task,
    "  - treating active PR #542 as a merged native-v2 consumer or cutover",
    "  - treating the producer package associated with PR #542 as proof of a native-v2 consumer, cutover or production activation",
    "status-independent rejected hypothesis",
)
task = replace_once(
    task,
    """  - command: bounded OPA-AUD-731-001 remediation
    result: PASS
    evidence: stale evidence-baseline and outage wording is scoped to retained legacy paths while the delivered Gateway path, production unknowns and five-path effective scope are preserved
blockers:
  - fresh independent re-audit of the remediated exact head not yet performed
  - final exact-head workflow generation after remediation not yet terminal
next_action: After the remediated exact head is terminal-green, create a fresh independent audit Issue for a validator that did not implement this repair; then reconcile zero-thread state and merge gates.""",
    """  - command: bounded OPA-AUD-731-001 remediation
    result: PASS
    evidence: stale evidence-baseline and outage wording is scoped to retained legacy paths while the delivered Gateway path, production unknowns and five-path effective scope are preserved
  - command: independent re-audit Issue #750 / PR review 4875167020
    result: FAIL
    evidence: OPA-ARCH-20260806-001-AUDIT-02 proved that SYSTEM_ARCHITECTURE.md and GAME_GATEWAY_IDENTITY_CONTRACT.md encoded transient `Active PR #542` liveness wording
  - command: bounded OPA-ARCH-20260806-001-AUDIT-02 remediation
    result: PASS
    evidence: both canonical documents now reference the producer package associated with PR #542 as immutable evidence context while preserving disabled-by-default, producer-only, no-consumer, no-cutover and no-production-activation invariants
blockers:
  - fresh independent audit of the successor exact head not yet performed
  - final exact-head workflow generation after audit02 remediation not yet terminal
next_action: After the successor exact head is terminal-green, create a fresh independent audit Issue for a validator that did not implement this repair; then reconcile zero-thread state and merge gates.""",
    "validation, blockers and next action",
)
task = replace_once(task, "  generation: 4", "  generation: 5", "recovery generation")
task = replace_once(task, "  session_id: chatgpt-20260806T1512+0200-game-auth-topology-audit-remediation", "  session_id: chatgpt-20260806T1555+0200-game-auth-topology-audit02-remediation", "recovery session")
task = replace_once(task, "  checkpointed_at: 2026-08-06T15:12:00+02:00", "  checkpointed_at: 2026-08-06T15:55:00+02:00", "recovery checkpointed_at")
task = replace_once(task, "  last_progress_at: 2026-08-06T15:12:00+02:00", "  last_progress_at: 2026-08-06T15:55:00+02:00", "recovery last_progress_at")
task = replace_once(task, "  phase: audit_remediation_validate", "  phase: audit02_remediation_validate", "recovery phase")
task = replace_once(task, "  active_operation: apply bounded OPA-AUD-731-001 documentation remediation", "  active_operation: apply bounded OPA-ARCH-20260806-001-AUDIT-02 documentation remediation", "recovery operation")
task = replace_once(task, "  check_generation: audit_remediation", "  check_generation: audit02_remediation", "recovery check generation")
task = replace_once(task, "  resume_condition: PR #731 contains the bounded remediation and no conflicting writer owns the five declared paths", "  resume_condition: PR #731 contains the bounded audit02 remediation and no conflicting writer owns the five declared paths", "recovery resume condition")
task = replace_once(task, "  next_action: Verify the remediated exact-head CI, then route the unchanged five-path diff to a different fresh independent validator.", "  next_action: Verify the audit02-remediated exact-head CI, then route the unchanged five-path diff to a different fresh independent validator.", "recovery next action")

system_path.write_text(system, encoding="utf-8")
gateway_path.write_text(gateway, encoding="utf-8")
task_path.write_text(task, encoding="utf-8")
