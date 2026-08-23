from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
LEGACY = "blakinio/Oteryn-Platform"
CURRENT = "Oteryn/Oteryn-Platform"

CURRENT_AUTHORITY_PATHS = (
    Path(".github"),
    Path("SECURITY.md"),
    Path("docs/operations"),
    Path("docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md"),
    Path("docs/agents/PROJECT_LANES.json"),
    Path("docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md"),
    Path("docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md"),
    Path("docs/agents/REMEDIATION_AUDIT_RISK_GATE.md"),
    Path("docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md"),
    Path("docs/agents/REPAIR_PR_ECONOMY.md"),
    Path("docs/agents/SHORT_PROGRAM_INVOCATIONS.md"),
    Path("tools/agents/historical_work_reconciliation.py"),
)
def _iter_files(path: Path):
    absolute = ROOT / path
    if absolute.is_file():
        yield absolute
        return
    if absolute.is_dir():
        yield from (candidate for candidate in absolute.rglob("*") if candidate.is_file())


def test_current_authority_surfaces_do_not_use_legacy_repository_coordinate():
    offenders: list[str] = []
    for relative in CURRENT_AUTHORITY_PATHS:
        for path in _iter_files(relative):
            try:
                text = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue
            if LEGACY in text:
                offenders.append(path.relative_to(ROOT).as_posix())

    assert not offenders, "legacy repository coordinate remains active in: " + ", ".join(offenders)


def test_root_governance_names_current_repository():
    root_agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
    assert CURRENT in root_agents
