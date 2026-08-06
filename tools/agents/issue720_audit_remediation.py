from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


def replace_first(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"{label}: required match not found")
    return text.replace(old, new, 1)


auth_path = Path("docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md")
task_path = Path("docs/agents/tasks/active/OTERYN-20260806-game-auth-topology-reconciliation.md")

auth = auth_path.read_text(encoding="utf-8")
task = task_path.read_text(encoding="utf-8")

auth = replace_once(
    auth,
    "- Current state: Platform-owned web Identity is implemented for registration, framework-hashed credentials, login/logout, revocable web sessions, password recovery/change and opt-in web MFA. It remains separate from Canary/login-server reusable credentials and is not yet the authoritative game-login credential verifier.",
    "- Current state: Platform-owned web Identity is implemented for registration, framework-hashed credentials, login/logout, revocable web sessions, password recovery/change and opt-in web MFA. It is authoritative for the delivered Oteryn Game Gateway path, but it is not yet the sole global game-login credential authority while native Canary, external login-server and other legacy password/session paths may remain reachable.",
    "Platform evidence baseline",
)

auth = replace_once(
    auth,
    """### Platform outage — CURRENTLY NOT AUTHORITATIVE FOR GAME AUTHENTICATION

Because Oteryn Platform is not yet in the game authentication path, its outage does not currently prevent native Canary or external login-server authentication.

### DERIVED target implication

If Platform Identity becomes the sole credential authority, its availability becomes part of the login critical path unless the architecture uses narrowly scoped pre-issued authorizations with explicit expiry/failure semantics.""",
    """### Platform outage — PATH-DEPENDENT

For the delivered Oteryn Gateway path, Oteryn Platform Identity is in the game-authentication critical path: ticket issuance, private ticket redeem and private login-context dependencies fail closed when Platform is unavailable.

Native Canary, external login-server and other retained legacy password/session paths may still authenticate independently when deployed and reachable. Therefore a Platform outage is not yet a proven global game-login shutdown condition, and exact production exposure or isolation of those alternate paths remains `UNKNOWN`.

### DERIVED sole-authority implication

If Platform Identity becomes the sole global credential authority, its availability becomes part of every supported login critical path unless the architecture uses narrowly scoped pre-issued authorizations with explicit expiry/failure semantics.""",
    "Platform outage semantics",
)

task = replace_first(
    task,
    "updated_at: 2026-08-06T12:49:00+02:00",
    "updated_at: 2026-08-06T15:12:00+02:00",
    "checkpoint updated_at",
)
task = replace_first(
    task,
    "last_progress_at: 2026-08-06T12:49:00+02:00",
    "last_progress_at: 2026-08-06T15:12:00+02:00",
    "checkpoint last_progress_at",
)
task = replace_once(
    task,
    """status: ready
phase: validate
session_id: none
session_role: none
execution_mode: github
execution_reason: implementation and the checkpoint-contract repair are complete; a fresh independent validator must now audit the exact final diff while exact-head CI runs""",
    """status: validating
phase: audit_remediation_validate
session_id: chatgpt-20260806T1512+0200-game-auth-topology-audit-remediation
session_role: implementer
execution_mode: github
execution_reason: independent audit review 4874934896 identified one high-severity internal contradiction; apply the smallest documentation-only repair and return the new exact head to CI and a different fresh validator""",
    "checkpoint execution state",
)
task = replace_once(
    task,
    "repair_cycles_for_current_gate: 1",
    "repair_cycles_for_current_gate: 2",
    "repair cycle counter",
)
task = replace_once(
    task,
    "  - The required context_routes and owned_paths lists were added on ef749a8e6bbdc2964429305513be24500927c946; Agent Governance run 31094598548 then passed.",
    """  - The required context_routes and owned_paths lists were added on ef749a8e6bbdc2964429305513be24500927c946; Agent Governance run 31094598548 then passed.
  - Independent audit Issue #737 and PR review 4874934896 recorded high finding OPA-AUD-731-001: the current overlay was contradicted by stale later wording that said Platform was not in the game-authentication path.""",
    "audit finding evidence",
)
task = replace_once(
    task,
    """blockers:
  - fresh independent documentation audit not yet performed
  - final exact-head workflow generation not yet terminal
next_action: A fresh independent validator claims the exact-head audit Issue linked from PR #731, audits the unchanged five-path diff and records PASS or exact findings; final exact-head CI and zero-thread state are then reconciled before merge.""",
    """  - command: independent audit Issue #737 / PR review 4874934896
    result: FAIL
    evidence: OPA-AUD-731-001 proved contradictory Platform-authority and outage wording in AUTH_GAME_LOGIN_CONTRACT.md
  - command: bounded OPA-AUD-731-001 remediation
    result: PASS
    evidence: stale evidence-baseline and outage wording is scoped to retained legacy paths while the delivered Gateway path, production unknowns and five-path effective scope are preserved
blockers:
  - fresh independent re-audit of the remediated exact head not yet performed
  - final exact-head workflow generation after remediation not yet terminal
next_action: After the remediated exact head is terminal-green, create a fresh independent audit Issue for a validator that did not implement this repair; then reconcile zero-thread state and merge gates.""",
    "validation and next action",
)

task = replace_once(task, "  generation: 3", "  generation: 4", "recovery generation")
task = replace_once(
    task,
    "  session_id: none",
    "  session_id: chatgpt-20260806T1512+0200-game-auth-topology-audit-remediation",
    "recovery session",
)
task = replace_once(
    task,
    "  checkpointed_at: 2026-08-06T12:49:00+02:00",
    "  checkpointed_at: 2026-08-06T15:12:00+02:00",
    "recovery checkpointed_at",
)
task = replace_once(
    task,
    "  last_progress_at: 2026-08-06T12:49:00+02:00",
    "  last_progress_at: 2026-08-06T15:12:00+02:00",
    "recovery last_progress_at",
)
task = replace_once(task, "  phase: validate", "  phase: audit_remediation_validate", "recovery phase")
task = replace_once(
    task,
    "  active_operation: none",
    "  active_operation: apply bounded OPA-AUD-731-001 documentation remediation",
    "recovery operation",
)
task = replace_once(
    task,
    "  check_generation: final_audit_target",
    "  check_generation: audit_remediation",
    "recovery check generation",
)
task = replace_once(
    task,
    "  resume_condition: PR #731 head is unchanged and no conflicting auditor owns the audit Issue",
    "  resume_condition: PR #731 contains the bounded remediation and no conflicting writer owns the five declared paths",
    "recovery resume condition",
)
task = replace_once(
    task,
    "  next_action: Claim and execute the independent exact-head documentation audit, then reconcile final CI and merge gates.",
    "  next_action: Verify the remediated exact-head CI, then route the unchanged five-path diff to a different fresh independent validator.",
    "recovery next action",
)

required_auth_markers = [
    "### Platform outage — PATH-DEPENDENT",
    "It is authoritative for the delivered Oteryn Game Gateway path",
    "PRODUCTION_PROVEN: false",
    "global_legacy_path_retirement: UNKNOWN",
]
for marker in required_auth_markers:
    if marker not in auth:
        raise SystemExit(f"missing required auth marker: {marker}")

for stale in [
    "### Platform outage — CURRENTLY NOT AUTHORITATIVE FOR GAME AUTHENTICATION",
    "Because Oteryn Platform is not yet in the game authentication path",
]:
    if stale in auth:
        raise SystemExit(f"stale contradictory marker remains: {stale}")

auth_path.write_text(auth, encoding="utf-8")
task_path.write_text(task, encoding="utf-8")
