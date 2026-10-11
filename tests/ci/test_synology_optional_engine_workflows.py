from __future__ import annotations

import pathlib

WORKFLOWS = pathlib.Path(__file__).resolve().parents[2] / ".github" / "workflows"
PROBE = "source deploy/synology/scripts/engine-topology.sh; oteryn_engine_enabled"


def _text(name: str) -> str:
    return (WORKFLOWS / name).read_text()


def test_workflows_gate_the_legacy_image_input_on_topology() -> None:
    deploy = _text("deploy-synology-staging.yml")
    assert f"engine_enabled=\"$(bash -c '{PROBE}')\"" in deploy
    assert "must be empty because the compose topology has no" in deploy
    assert "_dispatch_args=(-f" in _text("build-synology-staging-images.yml")
    assert "required_names+=(" in _text("recover-synology-staging-schema.yml")
    assert 'if [[ "$engine_enabled" == 1 ]]; then' in _text("character-bazaar-staging-control.yml")
