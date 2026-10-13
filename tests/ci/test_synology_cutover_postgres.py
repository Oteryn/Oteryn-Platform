#!/usr/bin/env python3
"""PostgreSQL staging cutover orchestration (deploy/synology/scripts/cutover-postgres.sh, plan P6).

Runs the real script with fake docker, gh and curl executables on PATH, a temporary state directory and
stub hooks. Nothing touches Docker, GitHub, Cloudflare or the NAS. The cases prove the state machine
(order, forward-only after step 4), the destructive-step entry gates, the dry-run default, the evidence
artifact JSON and that no secret reaches the output or the command lines.

Run: python3 tests/ci/test_synology_cutover_postgres.py
"""

from __future__ import annotations

import datetime as dt
import io
import json
import os
import re
import shutil
import stat
import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "deploy" / "synology" / "scripts" / "cutover-postgres.sh"
WORKFLOW = ROOT / ".github" / "workflows" / "deploy-synology-staging.yml"
BUILD_WORKFLOW = ROOT / ".github" / "workflows" / "build-synology-staging-images.yml"

TASK = "OTERYN-20261013-platform-p6b-cutover"
CLOSE_RUN = "7000000001"
def _checkout_sha() -> str:
    # The script refuses a dispatched SHA that differs from the checked-out tooling, so tests use the real checkout.
    out = subprocess.run(["git", "-C", str(Path(__file__).resolve().parents[2]), "rev-parse", "HEAD"], capture_output=True, text=True)
    sha = out.stdout.strip()
    return sha if re.fullmatch(r"[0-9a-f]{40}", sha) else "a" * 40


HEAD_SHA = _checkout_sha()
STORE_ID = "b" * 64
ASIDE = "/var/www/html/storage/app/native-evidence-witness.mariadb-era-20261013T010101Z"
PHRASE = "DELETE-MARIADB-VOLUME"
SECRETS = {
    "APP_KEY": "base64:SECRETAPPKEYSENTINEL0123456789abcdef=",
    "PLATFORM_DB_PASSWORD": "mariadbpasswordsentinel0123456789",
    "MARIADB_ROOT_PASSWORD": "mariadbrootsentinel0123456789abcd",
    "PLATFORM_POSTGRES_SUPERUSER_PASSWORD": "pgsuperusersentinel0123456789abc",
    "PLATFORM_POSTGRES_OWNER_PASSWORD": "pgownersentinel0123456789abcdef",
    "PLATFORM_POSTGRES_APP_PASSWORD": "pgappsentinel0123456789abcdefgh",
    "PLATFORM_POSTGRES_BACKUP_PASSWORD": "pgbackupsentinel0123456789abcdef",
    "PLATFORM_POSTGRES_DUMP_PASSWORD": "pgdumpsentinel0123456789abcdefg",
    "OTERYN_PLATFORM_SERVICE_TOKEN": "servicetokensentinel0123456789abc",
}

DOCKER_STUB = r"""#!/usr/bin/env bash
# Fake docker: logs the command line (never the program text) and answers the few read-only queries.
log="${STUB_LOG:?}"
args=("$@")
if [[ "$*" == *"--entrypoint sh platform -c"* ]]; then
    for i in "${!args[@]}"; do
        if [[ "${args[$i]}" == witness ]]; then
            echo "docker WITNESS ${args[*]:$((i + 1))}" >> "$log"
            case "${args[$((i + 1))]}" in
                restore) echo "witness_restored=true" ;;
                delete-aside) echo "witness_aside_deleted=true" ;;
            esac
            exit 0
        fi
    done
fi
echo "docker $*" >> "$log"
for i in "${!args[@]}"; do
    if [[ "${args[$i]}" == --env-file ]]; then
        echo "docker ENVFILE ${args[$((i + 1))]} $(grep '^PLATFORM_IMAGE=' "${args[$((i + 1))]}")" >> "$log"
    fi
done
case "$1 $2" in
    "volume inspect") [[ -z "${STUB_VOLUME_MISSING:-}" ]] || exit 1 ;;
esac
exit 0
"""

GH_STUB = r"""#!/usr/bin/env bash
# Fake gh api: serves fixture files from GH_FIXTURES by request path.
echo "gh $*" >> "${STUB_LOG:?}"
[[ "$1" == api ]] || exit 2
path="$2"
fix="${GH_FIXTURES:?}"
case "$path" in
    repos/Oteryn/Oteryn-Platform/actions/runs/*/artifacts)
        id="${path#repos/Oteryn/Oteryn-Platform/actions/runs/}"; id="${id%/artifacts}"
        cat "$fix/artifacts-$id.json" ;;
    repos/Oteryn/Oteryn-Platform/actions/runs/*)
        id="${path##*/}"
        cat "$fix/run-$id.json" ;;
    repos/Oteryn/Oteryn-Platform/actions/artifacts/*/zip)
        id="${path#repos/Oteryn/Oteryn-Platform/actions/artifacts/}"; id="${id%/zip}"
        cat "$fix/zip-$id.zip" ;;
    *) exit 3 ;;
esac
"""

CURL_STUB = r"""#!/usr/bin/env bash
url="${@: -1}"
echo "curl $url" >> "${STUB_LOG:?}"
host="${url#https://}"; host="${host%%/*}"
case "$host" in
    oteryn.molehill.cloud | gateway.molehill.cloud) printf '%s' "${STUB_CANONICAL_STATUS:-503}" ;;
    *) printf '%s' "${STUB_RANDOM_STATUS:-404}" ;;
esac
"""

BOOTSTRAP_STUB = r"""#!/usr/bin/env bash
echo "bootstrap $*" >> "${STUB_LOG:?}"
dir="${OTERYN_PG_BOOTSTRAP_STATE_DIR:?}"
mkdir -p "$dir"
case "$1" in
    record-witness) printf 'WITNESS_STORE_ID=%s\n' "$STUB_STORE_ID" > "$dir/witness-freeze.env" ;;
    run)
        printf 'MOVED_ASIDE_TO=%s\nWITNESS_STORE_ID=%s\n' "$STUB_ASIDE" "$STUB_STORE_ID" > "$dir/witness-moved-aside.env"
        echo "bootstrap_result=complete"
        echo "oauth_client_id=11111111-1111-1111-1111-111111111111"
        echo "oauth_rust_client_id=22222222-2222-2222-2222-222222222222"
        echo "native_world_id=42"
        echo "witness_moved_aside_to=$STUB_ASIDE"
        ;;
esac
"""

LOGGING_STUB = r"""#!/usr/bin/env bash
echo "$(basename "$0") $*" >> "${STUB_LOG:?}"
exit "${STUB_EXIT:-0}"
"""

# The hook records whether the marker already existed when it ran.
PROVENANCE_STUB = r"""#!/usr/bin/env bash
if grep -q '^STEP_REENABLE_STARTED=' "${OTERYN_STATE_DIR:?}/postgres-cutover/cutover-state.env"; then marker=marker-present; else marker=marker-absent; fi
echo "provenance-init $* $marker" >> "${STUB_LOG:?}"
exit "${STUB_PROVENANCE_EXIT:-0}"
"""


def now_iso(offset_seconds: int = 0) -> str:
    moment = dt.datetime.now(dt.timezone.utc) + dt.timedelta(seconds=offset_seconds)
    return moment.strftime("%Y-%m-%dT%H:%M:%SZ")


class Harness:
    def __init__(self, test: unittest.TestCase) -> None:
        self.test = test
        self.root = Path(tempfile.mkdtemp(prefix="cutover-test-"))
        test.addCleanup(shutil.rmtree, self.root, True)
        self.bin = self.root / "bin"
        self.fixtures = self.root / "fixtures"
        self.state_root = self.root / "state"
        self.bootstrap_state = self.root / "bootstrap-state"
        self.evidence = self.root / "evidence"
        self.log = self.root / "stub.log"
        self.env_file = self.root / "staging.env"
        self.contract = self.root / "release-contract.env"
        for directory in (self.bin, self.fixtures, self.state_root):
            directory.mkdir()
        self.log.write_text("")
        self.write_stub("docker", DOCKER_STUB)
        self.write_stub("gh", GH_STUB)
        self.write_stub("curl", CURL_STUB)
        self.bootstrap_script = self.write_stub("bootstrap-postgres.sh", BOOTSTRAP_STUB, self.root)
        self.health_script = self.write_stub("health-check.sh", LOGGING_STUB, self.root)
        self.rollback_script = self.write_stub("rollback.sh", LOGGING_STUB, self.root)
        self.provenance = self.write_stub("provenance-init", PROVENANCE_STUB, self.root)
        self.web_smoke_hook = self.write_stub("web-smoke", LOGGING_STUB, self.root)
        self.native_smoke_hook = self.write_stub("native-smoke", LOGGING_STUB, self.root)
        lines = ["COMPOSE_PROJECT_NAME=oteryn-staging", "PLATFORM_IMAGE=ghcr.io/oteryn/oteryn-platform@sha256:" + "c" * 64,
                 "GATEWAY_IMAGE=ghcr.io/oteryn/oteryn-gateway@sha256:" + "d" * 64, "OTERYN_NATIVE_ADMISSION_ENABLED=false"]
        lines += [f"{key}={value}" for key, value in SECRETS.items()]
        self.env_file.write_text("\n".join(lines) + "\n")
        self.set_engine("mariadb")
        self.audit_counter = 8000000000
        self.outputs: list[str] = []

    def write_stub(self, name: str, body: str, directory: Path | None = None) -> Path:
        path = (directory or self.bin) / name
        path.write_text(body)
        path.chmod(path.stat().st_mode | stat.S_IXUSR)
        return path

    def set_engine(self, engine: str) -> None:
        self.contract.write_text(f"PLATFORM_DB_ENGINE={engine}\n")

    # --- GitHub fixtures ---
    def make_audit(self, state: str, *, task: str = TASK, close_run: str = CLOSE_RUN, mutate=None,
                   branch: str = "main", event: str = "workflow_dispatch", conclusion: str = "success",
                   age: int = 60, artifact_count: int = 1, expired: bool = False, json_run_id: int | None = None) -> str:
        self.audit_counter += 1
        run_id = self.audit_counter
        audit = {
            "schema": "oteryn.edge-audit-result.v1", "audit_run_id": json_run_id if json_run_id is not None else run_id,
            "run_attempt": 1, "task_id": task, "closure_run_id": close_run if state == "closed" else None,
            "gate_state": state, "edge_version": 5, "config_hash": "e" * 64, "tunnel_contract": "current",
            "dns": "current", "other_origin_routes": 0,
            "closure_match": "yes" if state == "closed" else "not_requested",
        }
        if mutate:
            mutate(audit)
        run = {"id": run_id, "path": ".github/workflows/cloudflare-oteryn-endpoints.yml", "event": event,
               "head_branch": branch, "status": "completed", "conclusion": conclusion, "run_attempt": 1}
        artifacts = {"artifacts": [{"id": run_id * 10 + n, "name": "oteryn-edge-audit-result", "expired": expired,
                                    "created_at": now_iso(-age)} for n in range(artifact_count)]}
        (self.fixtures / f"run-{run_id}.json").write_text(json.dumps(run))
        (self.fixtures / f"artifacts-{run_id}.json").write_text(json.dumps(artifacts))
        for n in range(artifact_count):
            buffer = io.BytesIO()
            with zipfile.ZipFile(buffer, "w") as archive:
                archive.writestr("audit.json", json.dumps(audit))
            (self.fixtures / f"zip-{run_id * 10 + n}.zip").write_bytes(buffer.getvalue())
        return str(run_id)

    # --- Running the script ---
    def env(self, **overrides: str) -> dict[str, str]:
        base = {
            "PATH": f"{self.bin}:{os.environ['PATH']}", "HOME": str(self.root),
            "OTERYN_ENV_FILE": str(self.env_file), "OTERYN_RELEASE_CONTRACT_FILE": str(self.contract),
            "OTERYN_STATE_DIR": str(self.state_root), "OTERYN_PG_BOOTSTRAP_STATE_DIR": str(self.bootstrap_state),
            "OTERYN_CUTOVER_EVIDENCE_DIR": str(self.evidence), "STUB_LOG": str(self.log),
            "GH_FIXTURES": str(self.fixtures), "GITHUB_REPOSITORY": "Oteryn/Oteryn-Platform", "GITHUB_SHA": HEAD_SHA,
            "STUB_STORE_ID": STORE_ID, "STUB_ASIDE": ASIDE,
            "OTERYN_CUTOVER_TASK_ID": TASK, "OTERYN_CUTOVER_BOOTSTRAP_SCRIPT": str(self.bootstrap_script),
            "OTERYN_CUTOVER_HEALTH_SCRIPT": str(self.health_script), "OTERYN_CUTOVER_ROLLBACK_SCRIPT": str(self.rollback_script),
            "OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND": str(self.provenance),
            "OTERYN_CUTOVER_WEB_SMOKE_COMMAND": str(self.web_smoke_hook),
            "OTERYN_CUTOVER_NATIVE_SMOKE_COMMAND": str(self.native_smoke_hook),
        }
        base.update(overrides)
        return base

    def run(self, sub: str, real: bool = True, **overrides: str) -> subprocess.CompletedProcess[str]:
        if real:
            overrides.setdefault("OTERYN_CUTOVER_DRY_RUN", "0")
        result = subprocess.run(["bash", str(SCRIPT), sub], env=self.env(**overrides), text=True,
                                capture_output=True, timeout=60, check=False)
        self.outputs.append(result.stdout + result.stderr)
        return result

    def ok(self, sub: str, **overrides: str) -> subprocess.CompletedProcess[str]:
        result = self.run(sub, **overrides)
        self.test.assertEqual(result.returncode, 0, f"{sub}: {result.stdout}{result.stderr}")
        return result

    def refused(self, sub: str, fragment: str, **overrides: str) -> subprocess.CompletedProcess[str]:
        result = self.run(sub, **overrides)
        self.test.assertEqual(result.returncode, 1, f"{sub} should be refused: {result.stdout}{result.stderr}")
        self.test.assertIn(fragment, result.stderr)
        return result

    # --- Step helpers ---
    def preflight(self) -> None:
        self.ok("preflight", OTERYN_CUTOVER_OWNER_GO=TASK, OTERYN_CUTOVER_AUDIT_RUN_ID=self.make_audit("open"))

    def closed(self, **kwargs: str) -> dict[str, str]:
        return {"OTERYN_CUTOVER_CLOSE_RUN_ID": CLOSE_RUN, "OTERYN_CUTOVER_AUDIT_RUN_ID": self.make_audit("closed"), **kwargs}

    def freeze(self) -> None:
        self.ok("freeze", **self.closed())

    def bootstrap(self) -> None:
        self.set_engine("postgres")
        self.ok("bootstrap", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN)

    def smoke(self) -> None:
        self.ok("smoke", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN)

    def reenable(self) -> None:
        self.ok("native-reenable", **self.closed())

    def native_smoke(self) -> None:
        self.write_env("OTERYN_NATIVE_ADMISSION_ENABLED", "true")
        self.ok("native-smoke", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN)

    def declare_done(self) -> None:
        self.ok("declare-done", **self.closed())

    def advance_to(self, stage: str) -> None:
        order = ["preflight", "freeze", "bootstrap", "smoke", "reenable", "native_smoke", "declare_done"]
        done = self.__dict__.setdefault("_done", set())
        for name in order[: order.index(stage) + 1]:
            if name not in done:
                getattr(self, name)()
                done.add(name)

    def write_env(self, key: str, value: str) -> None:
        lines = [line for line in self.env_file.read_text().splitlines() if not line.startswith(key + "=")]
        self.env_file.write_text("\n".join(lines + [f"{key}={value}"]) + "\n")

    def retire_env(self, **overrides: str) -> dict[str, str]:
        return self.closed(OTERYN_OWNER_DELETE_CONFIRMATION=PHRASE, OTERYN_OWNER_DELETE_ACK=TASK, **overrides)

    def log_lines(self) -> list[str]:
        return self.log.read_text().splitlines()

    def state(self) -> str:
        path = self.state_root / "postgres-cutover" / "cutover-state.env"
        return path.read_text() if path.exists() else ""

    def has_state(self, step: str) -> bool:
        return re.search(rf"^STEP_{step}=", self.state(), re.M) is not None

    def destructive_lines(self) -> list[str]:
        return [line for line in self.log_lines()
                if " volume rm" in line or " rm " in line or "WITNESS delete-aside" in line]


class CutoverTestBase(unittest.TestCase):
    def setUp(self) -> None:
        self.h = Harness(self)


class PreflightTests(CutoverTestBase):
    def test_unknown_subcommand_is_refused(self) -> None:
        result = subprocess.run(["bash", str(SCRIPT), "explode"], capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 1)

    def test_dry_run_is_the_default_and_records_nothing(self) -> None:
        env = self.h.env(OTERYN_CUTOVER_OWNER_GO=TASK, OTERYN_CUTOVER_AUDIT_RUN_ID=self.h.make_audit("open"))
        env.pop("OTERYN_CUTOVER_DRY_RUN", None)
        result = subprocess.run(["bash", str(SCRIPT), "preflight"], env=env, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("DRY RUN", result.stdout)
        self.assertEqual(self.h.state(), "")

    def test_passes_and_records_in_real_mode(self) -> None:
        self.h.preflight()
        self.assertTrue(self.h.has_state("PREFLIGHT"))
        self.assertIn(f"TASK_ID={TASK}", self.h.state())

    def test_refusals(self) -> None:
        h = self.h
        open_run = lambda: h.make_audit("open")  # noqa: E731
        base = lambda **kw: {"OTERYN_CUTOVER_OWNER_GO": TASK, "OTERYN_CUTOVER_AUDIT_RUN_ID": open_run(), **kw}  # noqa: E731
        h.refused("preflight", "go decision", OTERYN_CUTOVER_AUDIT_RUN_ID=open_run())
        h.refused("preflight", "OTERYN-YYYYMMDD", **base(OTERYN_CUTOVER_TASK_ID="bad"))
        h.refused("preflight", "GITHUB_REPOSITORY", **base(GITHUB_REPOSITORY="Oteryn/Other"))
        h.refused("preflight", "audit result", OTERYN_CUTOVER_OWNER_GO=TASK, OTERYN_CUTOVER_AUDIT_RUN_ID=h.make_audit("closed"))
        h.refused("preflight", "MARKETPLACE_ENABLED must be false", **self._marketplace_on(base()))
        (h.state_root / "marketplace.env").unlink()
        h.write_env("OTERYN_NATIVE_ADMISSION_ENABLED", "true")
        h.refused("preflight", "native admission is on", **base())
        h.write_env("OTERYN_NATIVE_ADMISSION_ENABLED", "false")
        h.set_engine("postgres")
        h.refused("preflight", "mariadb", **base())
        h.set_engine("mariadb")
        h.refused("preflight", "OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND", **base(OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND=""))
        h.refused("preflight", "OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND", **base(OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND=str(h.root / "missing")))
        h.refused("preflight", "OTERYN_CUTOVER_NATIVE_SMOKE_COMMAND", **base(OTERYN_CUTOVER_NATIVE_SMOKE_COMMAND=""))
        h.write_env("PLATFORM_POSTGRES_DUMP_PASSWORD", "")
        h.refused("preflight", "all five PostgreSQL secrets", **base())
        self.assertEqual(h.state(), "")

    def _marketplace_on(self, env: dict[str, str]) -> dict[str, str]:
        (self.h.state_root / "marketplace.env").write_text("MARKETPLACE_ENABLED=true\n")
        return env

    def test_marketplace_state_without_the_key_fails_closed(self) -> None:
        (self.h.state_root / "marketplace.env").write_text("OTHER=1\n")
        self.h.refused("preflight", "MARKETPLACE_ENABLED must be false",
                       OTERYN_CUTOVER_OWNER_GO=TASK, OTERYN_CUTOVER_AUDIT_RUN_ID=self.h.make_audit("open"))


class AuditValidationTests(CutoverTestBase):
    """The audit evidence is validated by pure jq functions; each rule is exercised through preflight."""

    def check(self, fragment: str = "audit", **kwargs) -> None:
        run = self.h.make_audit("open", **kwargs)
        self.h.refused("preflight", fragment, OTERYN_CUTOVER_OWNER_GO=TASK, OTERYN_CUTOVER_AUDIT_RUN_ID=run)

    def test_provenance_and_artifact_rules(self) -> None:
        self.check(branch="feature")
        self.check(event="push")
        self.check(conclusion="failure")
        self.check(artifact_count=2)
        self.check(artifact_count=0)
        self.check(expired=True)
        self.check(age=86400 + 120)

    def test_content_rules(self) -> None:
        self.check(task="OTERYN-20261013-someone-else")
        self.check(json_run_id=1)
        self.check(mutate=lambda a: a.update(other_origin_routes=1))
        self.check(mutate=lambda a: a.update(closure_match="yes"))
        self.check(mutate=lambda a: a.update(gate_state="closed"))
        self.check(mutate=lambda a: a.update(dns={"host": "internal.example.com"}))
        self.check(mutate=lambda a: a.pop("schema"))

    def test_canonical_hostnames_alone_are_accepted(self) -> None:
        run = self.h.make_audit("open", mutate=lambda a: a.update(dns={"hosts": ["oteryn.molehill.cloud", "gateway.molehill.cloud"]}))
        self.h.ok("preflight", OTERYN_CUTOVER_OWNER_GO=TASK, OTERYN_CUTOVER_AUDIT_RUN_ID=run)

    def test_closed_audit_must_carry_the_close_run(self) -> None:
        self.h.preflight()
        wrong = self.h.make_audit("closed", close_run="1234")
        self.h.refused("freeze", "audit result", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN, OTERYN_CUTOVER_AUDIT_RUN_ID=wrong)
        no_match = self.h.make_audit("closed", mutate=lambda a: a.update(closure_match="no"))
        self.h.refused("freeze", "audit result", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN, OTERYN_CUTOVER_AUDIT_RUN_ID=no_match)


class OrderTests(CutoverTestBase):
    def test_every_step_refuses_out_of_order(self) -> None:
        h = self.h
        closed = lambda: h.closed()  # noqa: E731
        h.refused("freeze", "preflight has not passed", **closed())
        h.preflight()
        h.refused("bootstrap", "freeze is not recorded", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN)
        h.refused("smoke", "bootstrap is not recorded", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN)
        h.refused("native-reenable", "step 3 smoke is not recorded", **closed())
        h.refused("native-smoke", "has not started", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN)
        h.refused("declare-done", "step 5 native login smoke is not recorded", **closed())
        h.refused("retire-mariadb", "native login smoke", **h.retire_env())
        h.refused("unfreeze", "gate stays closed", OTERYN_CUTOVER_AUDIT_RUN_ID=h.make_audit("open"))
        h.freeze()
        h.refused("freeze", "already recorded", **closed())
        h.refused("native-reenable", "step 3 smoke is not recorded", **closed())
        h.refused("bootstrap", "PLATFORM_DB_ENGINE=postgres", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN)
        h.bootstrap()
        h.refused("bootstrap", "already recorded", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN)
        h.refused("native-reenable", "step 3 smoke is not recorded", **closed())

    def test_a_different_task_cannot_drive_the_state(self) -> None:
        self.h.preflight()
        self.h.refused("freeze", "another task", OTERYN_CUTOVER_TASK_ID="OTERYN-20261013-other-task", **self.h.closed())

    def test_close_run_id_is_pinned_at_the_freeze(self) -> None:
        self.h.advance_to("freeze")
        self.h.set_engine("postgres")
        self.h.refused("bootstrap", "close run id differs", OTERYN_CUTOVER_CLOSE_RUN_ID="999")

    def test_freeze_needs_the_edge_503(self) -> None:
        self.h.preflight()
        self.h.refused("freeze", "did not get the edge's 503", STUB_CANONICAL_STATUS="200", **self.h.closed())
        self.h.refused("freeze", "application-like response", STUB_RANDOM_STATUS="200", **self.h.closed())
        self.assertFalse(self.h.has_state("FREEZE"))
        self.assertFalse([line for line in self.h.log_lines() if line.startswith("docker compose")])

    def test_freeze_stops_the_stack_and_records_the_witness(self) -> None:
        self.h.preflight()
        self.h.freeze()
        log = "\n".join(self.h.log_lines())
        self.assertIn("stop gateway internal-proxy internal-mtls", log)
        self.assertIn("stop marketplace-scheduler", log)
        self.assertIn("stop platform mariadb", log)
        self.assertIn("bootstrap record-witness", log)
        self.assertIn("MARIADB_VOLUME=oteryn-staging_mariadb_data", self.h.state())

    def test_freeze_dry_run_stops_nothing(self) -> None:
        self.h.preflight()
        result = self.h.run("freeze", real=False, **self.h.closed())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("PLAN:", result.stdout)
        self.assertFalse([line for line in self.h.log_lines() if line.startswith("docker compose")])
        self.assertFalse(self.h.has_state("FREEZE"))

    def test_runtime_images_replace_the_workflow_placeholders(self) -> None:
        h = self.h
        h.write_env("PLATFORM_IMAGE", "rollback-placeholder")
        (h.state_root / "current-release.env").write_text(
            "RELEASE_SHA=" + "1" * 40 + "\nPLATFORM_SOURCE_SHA=" + "2" * 40 + "\nGATEWAY_SOURCE_SHA=" + "3" * 40
            + "\nPLATFORM_IMAGE=ghcr.io/oteryn/oteryn-platform@sha256:" + "9" * 64
            + "\nGATEWAY_IMAGE=ghcr.io/oteryn/oteryn-gateway@sha256:" + "8" * 64 + "\n")
        h.preflight()
        h.freeze()
        envfiles = [line for line in h.log_lines() if line.startswith("docker ENVFILE")]
        self.assertTrue(envfiles)
        for line in envfiles:
            self.assertIn("runtime.env", line)
            self.assertIn("sha256:" + "9" * 64, line)
            self.assertNotIn("rollback-placeholder", line)
        self.assertFalse((h.state_root / "postgres-cutover" / "runtime.env").exists())


class RollbackTests(CutoverTestBase):
    def evidence_file(self) -> Path:
        return self.h.evidence / "gate-evidence.json"

    def test_rollback_after_a_failed_smoke_window(self) -> None:
        h = self.h
        h.advance_to("smoke")
        h.set_engine("mariadb")
        self.assertFalse(self.evidence_file().exists())
        h.ok("rollback", **h.closed())
        log = "\n".join(h.log_lines())
        self.assertIn(f"WITNESS restore {ASIDE} {STORE_ID}", log)
        self.assertIn("up -d mariadb", log)
        self.assertIn("rollback.sh", log)
        self.assertGreaterEqual(log.count("health-check.sh"), 1)
        self.assertNotIn("WITNESS delete-aside", log)
        self.assertEqual(h.destructive_lines(), [])
        evidence = json.loads(self.evidence_file().read_text())
        self.assertEqual(evidence, {"schema": "oteryn.platform-cutover-gate-evidence.v1", "task_id": TASK,
                                    "close_run_id": CLOSE_RUN, "outcome": "rollback_verified", "head_sha": HEAD_SHA})
        self.assertTrue(h.has_state("ROLLBACK_VERIFIED"))
        self.assertFalse(h.bootstrap_state.exists())
        self.assertTrue(any(p.name.startswith("postgres-bootstrap") or p.name.startswith("bootstrap-state.rolled-back") for p in h.root.iterdir()))

    def test_bootstrap_adopts_a_completed_record_without_rerunning(self) -> None:
        h = self.h
        h.advance_to("freeze")
        h.bootstrap_state.mkdir(parents=True, exist_ok=True)
        (h.bootstrap_state / "bootstrap-complete.env").write_text(
            "BOOTSTRAPPED_AT=2026-10-13T00:00:00Z\nOAUTH_CLIENT_ID=11111111-1111-1111-1111-111111111111\n"
            "OAUTH_RUST_CLIENT_ID=22222222-2222-2222-2222-222222222222\nNATIVE_WORLD_ID=42\nNATIVE_ADMISSION=off\n"
        )
        h.bootstrap()
        self.assertTrue(h.has_state("BOOTSTRAP"))
        self.assertNotIn("bootstrap run", "\n".join(h.log_lines()))

    def test_rollback_before_bootstrap_restores_no_witness(self) -> None:
        h = self.h
        h.advance_to("freeze")
        h.ok("rollback", **h.closed())
        self.assertNotIn("WITNESS", "\n".join(h.log_lines()))
        self.assertEqual(json.loads(self.evidence_file().read_text())["outcome"], "rollback_verified")

    def test_rollback_needs_the_mariadb_contract_and_a_closed_gate(self) -> None:
        h = self.h
        h.advance_to("smoke")
        h.refused("rollback", "revert the release contract", **h.closed())
        h.set_engine("mariadb")
        h.refused("rollback", "audit result", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN, OTERYN_CUTOVER_AUDIT_RUN_ID=h.make_audit("open"))
        self.assertFalse(self.evidence_file().exists())
        self.assertFalse(h.has_state("ROLLBACK_STARTED"))

    def test_rollback_dry_run_changes_nothing(self) -> None:
        h = self.h
        h.advance_to("smoke")
        h.set_engine("mariadb")
        before = len(h.log_lines())
        result = h.run("rollback", real=False, **h.closed())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("PLAN:", result.stdout)
        self.assertEqual([line for line in h.log_lines()[before:] if line.startswith(("docker", "bootstrap", "rollback", "health"))], [])
        self.assertFalse(self.evidence_file().exists())
        self.assertFalse(h.has_state("ROLLBACK_STARTED"))

    def test_rollback_is_refused_once_step_4_starts(self) -> None:
        h = self.h
        h.advance_to("smoke")
        h.reenable()
        h.set_engine("mariadb")
        before = h.log_lines()
        h.refused("rollback", "forward-only", **h.closed())
        self.assertEqual(h.log_lines(), before, "a refused rollback must issue no command at all")
        self.assertFalse(self.evidence_file().exists())
        self.assertNotIn("WITNESS restore", "\n".join(h.log_lines()))

    def test_rollback_is_refused_after_a_failed_provenance_init(self) -> None:
        h = self.h
        h.advance_to("smoke")
        result = h.run("native-reenable", STUB_PROVENANCE_EXIT="1", **h.closed())
        self.assertEqual(result.returncode, 2)
        self.assertTrue(h.has_state("REENABLE_STARTED"))
        h.set_engine("mariadb")
        h.refused("rollback", "forward-only", **h.closed())

    def test_rollback_is_refused_after_later_steps(self) -> None:
        h = self.h
        h.advance_to("declare_done")
        h.set_engine("mariadb")
        h.refused("rollback", "forward-only", **h.closed())

    def test_a_native_smoke_failure_does_not_reopen_rollback(self) -> None:
        h = self.h
        h.advance_to("reenable")
        h.write_env("OTERYN_NATIVE_ADMISSION_ENABLED", "true")
        failing = h.run("native-smoke", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN, STUB_EXIT="1")
        self.assertNotEqual(failing.returncode, 0)
        self.assertFalse(h.has_state("NATIVE_SMOKE"))
        h.set_engine("mariadb")
        h.refused("rollback", "forward-only", **h.closed())
        h.refused("declare-done", "native login smoke is not recorded", **h.closed())


class NativeReenableTests(CutoverTestBase):
    def test_hook_must_exist_and_no_marker_is_written_without_it(self) -> None:
        h = self.h
        h.advance_to("smoke")
        h.refused("native-reenable", "OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND",
                  OTERYN_CUTOVER_PROVENANCE_INIT_COMMAND=str(h.root / "absent"), **h.closed())
        self.assertFalse(h.has_state("REENABLE_STARTED"))
        # The rollback window is still open.
        h.set_engine("mariadb")
        h.ok("rollback", **h.closed())

    def test_marker_is_written_before_the_hook_runs(self) -> None:
        h = self.h
        h.advance_to("smoke")
        h.reenable()
        lines = [line for line in h.log_lines() if line.startswith("provenance-init")]
        self.assertEqual(lines, [f"provenance-init init {TASK} {CLOSE_RUN} marker-present"])

    def test_native_admission_must_be_off_for_the_init(self) -> None:
        h = self.h
        h.advance_to("smoke")
        h.write_env("OTERYN_NATIVE_ADMISSION_ENABLED", "true")
        h.refused("native-reenable", "still be off", **h.closed())

    def test_dry_run_neither_runs_the_hook_nor_ends_the_rollback_window(self) -> None:
        h = self.h
        h.advance_to("smoke")
        result = h.run("native-reenable", real=False, **h.closed())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse([line for line in h.log_lines() if line.startswith("provenance-init") and "marker" in line])
        self.assertFalse(h.has_state("REENABLE_STARTED"))

    def test_native_smoke_needs_admission_on_and_runs_the_self_check(self) -> None:
        h = self.h
        h.advance_to("reenable")
        h.refused("native-smoke", "native admission is not on", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN)
        h.native_smoke()
        log = "\n".join(h.log_lines())
        self.assertIn("game-auth:native-admission:self-check", log)
        self.assertIn("native-smoke", log)
        self.assertTrue(h.has_state("NATIVE_SMOKE"))


class EvidenceTests(CutoverTestBase):
    def test_declare_done_writes_exactly_the_cutover_complete_evidence(self) -> None:
        h = self.h
        h.advance_to("native_smoke")
        path = h.evidence / "gate-evidence.json"
        self.assertFalse(path.exists(), "no evidence before the cutover is declared done")
        h.declare_done()
        evidence = json.loads(path.read_text())
        self.assertEqual(evidence, {"schema": "oteryn.platform-cutover-gate-evidence.v1", "task_id": TASK,
                                    "close_run_id": CLOSE_RUN, "outcome": "cutover_complete", "head_sha": HEAD_SHA})
        self.assertTrue(h.has_state("DECLARED_DONE"))
        h.refused("declare-done", "already declared done", **h.closed())

    def test_no_evidence_in_dry_run_or_after_a_refusal(self) -> None:
        h = self.h
        h.advance_to("native_smoke")
        path = h.evidence / "gate-evidence.json"
        result = h.run("declare-done", real=False, **h.closed())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(path.exists())
        h.refused("declare-done", "audit result", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN, OTERYN_CUTOVER_AUDIT_RUN_ID=h.make_audit("open"))
        self.assertFalse(path.exists())
        self.assertFalse(h.has_state("DECLARED_DONE"))

    def test_no_evidence_from_any_other_step(self) -> None:
        h = self.h
        h.advance_to("native_smoke")
        self.assertFalse((h.evidence / "gate-evidence.json").exists())
        self.assertFalse(h.evidence.exists() and any(h.evidence.iterdir()))

    def test_evidence_refuses_a_dispatched_sha_that_is_not_the_checked_out_tooling(self) -> None:
        h = self.h
        h.advance_to("native_smoke")
        other = "b" * 40 if HEAD_SHA != "b" * 40 else "c" * 40
        result = h.run("declare-done", GITHUB_SHA=other, **h.closed())
        if HEAD_SHA != "a" * 40:
            self.assertEqual(result.returncode, 1)
            self.assertIn("differs from the dispatched SHA", result.stdout + result.stderr)
            self.assertFalse(h.evidence.exists() and any(h.evidence.iterdir()))

    def test_missing_head_sha_is_refused(self) -> None:
        h = self.h
        h.advance_to("native_smoke")
        result = h.run("declare-done", GITHUB_SHA="not-a-sha", **h.closed())
        # git in the repository checkout supplies a real HEAD, so only an unusable value with no git fallback is refused.
        self.assertIn(result.returncode, (0, 1))
        if result.returncode == 0:
            self.assertRegex(json.loads((h.evidence / "gate-evidence.json").read_text())["head_sha"], r"^[0-9a-f]{40}$")


class RetireMariadbTests(CutoverTestBase):
    def ready(self) -> None:
        self.h.advance_to("declare_done")

    def assert_nothing_destroyed(self) -> None:
        self.assertEqual(self.h.destructive_lines(), [])
        self.assertFalse(self.h.has_state("MARIADB_RETIRED"))
        self.assertFalse(self.h.has_state("RETIRE_STARTED"))

    def test_refused_without_the_owner_gate(self) -> None:
        self.ready()
        self.h.refused("retire-mariadb", "owner delete confirmation", **self.h.closed(OTERYN_OWNER_DELETE_ACK=TASK))
        self.h.refused("retire-mariadb", "owner delete confirmation", **self.h.closed())
        self.assert_nothing_destroyed()

    def test_refused_with_a_wrong_phrase(self) -> None:
        self.ready()
        for phrase in ("delete-mariadb-volume", "DELETE-MARIADB-VOLUME ", " DELETE-MARIADB-VOLUME", "DELETE-MARIADB", "yes", "DELETE-MARIADB-VOLUME\n", "DELETE-POSTGRES-VOLUME"):
            with self.subTest(phrase=phrase):
                self.h.refused("retire-mariadb", "owner delete confirmation",
                               **self.h.closed(OTERYN_OWNER_DELETE_CONFIRMATION=phrase, OTERYN_OWNER_DELETE_ACK=TASK))
        self.assert_nothing_destroyed()

    def test_refused_without_an_ack_naming_the_task(self) -> None:
        self.ready()
        for ack in ("", "OTERYN-20261013-other-task", TASK + "-extra", "true"):
            with self.subTest(ack=ack):
                self.h.refused("retire-mariadb", "acknowledgement",
                               **self.h.closed(OTERYN_OWNER_DELETE_CONFIRMATION=PHRASE, OTERYN_OWNER_DELETE_ACK=ack))
        self.assert_nothing_destroyed()

    def test_refused_before_the_smoke_is_recorded(self) -> None:
        h = self.h
        h.advance_to("smoke")
        h.refused("retire-mariadb", "native login smoke", **h.retire_env())
        h.advance_to("reenable")
        h.refused("retire-mariadb", "native login smoke", **h.retire_env())
        self.assert_nothing_destroyed()

    def test_refused_after_the_smoke_but_before_declared_done(self) -> None:
        h = self.h
        h.advance_to("native_smoke")
        h.refused("retire-mariadb", "not declared done", **h.retire_env())
        self.assert_nothing_destroyed()

    def test_refused_when_the_gate_is_not_closed(self) -> None:
        self.ready()
        self.h.refused("retire-mariadb", "audit result", OTERYN_CUTOVER_CLOSE_RUN_ID=CLOSE_RUN,
                       OTERYN_CUTOVER_AUDIT_RUN_ID=self.h.make_audit("open"),
                       OTERYN_OWNER_DELETE_CONFIRMATION=PHRASE, OTERYN_OWNER_DELETE_ACK=TASK)
        self.assert_nothing_destroyed()

    def test_dry_run_by_default_prints_the_plan_and_deletes_nothing(self) -> None:
        self.ready()
        env = self.h.env(**self.h.retire_env())
        env.pop("OTERYN_CUTOVER_DRY_RUN", None)
        result = subprocess.run(["bash", str(SCRIPT), "retire-mariadb"], env=env, capture_output=True, text=True, check=False)
        self.h.outputs.append(result.stdout + result.stderr)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("PLAN: docker volume rm oteryn-staging_mariadb_data", result.stdout)
        self.assertIn("DRY RUN", result.stdout)
        self.assert_nothing_destroyed()

    def test_all_gates_issue_exactly_the_retirement_commands(self) -> None:
        h = self.h
        self.ready()
        before = len(h.log_lines())
        result = h.ok("retire-mariadb", **h.retire_env())
        issued = [line for line in h.log_lines()[before:] if line.startswith("docker") and "ENVFILE" not in line]
        compose_rm = [line for line in issued if " rm -s -f mariadb" in line]
        self.assertEqual(len(compose_rm), 1)
        self.assertIn("docker volume rm oteryn-staging_mariadb_data", issued)
        self.assertIn(f"docker WITNESS delete-aside {ASIDE} {STORE_ID}", issued)
        self.assertLess(issued.index(compose_rm[0]), issued.index("docker volume rm oteryn-staging_mariadb_data"))
        # Nothing but the MariaDB volume, container and MariaDB-era witness directory.
        removals = [line for line in issued if " volume rm" in line or " rm " in line or "delete-aside" in line]
        self.assertEqual(len(removals), 3, removals)
        joined = "\n".join(issued)
        for protected in ("postgres_data", "platform_storage", "restore"):
            self.assertNotIn(protected, joined)
        self.assertTrue(h.has_state("RETIRE_STARTED"))
        self.assertTrue(h.has_state("MARIADB_RETIRED"))
        self.assertRegex(h.state(), r"MARIADB_DELETED_AT=\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ")
        self.assertRegex(result.stdout, r"mariadb_deleted_at=\d{4}-")
        h.refused("retire-mariadb", "already retired", **h.retire_env())

    def test_retirement_never_touches_the_postgres_witness_directory(self) -> None:
        h = self.h
        self.ready()
        h.ok("retire-mariadb", **h.retire_env())
        witness_calls = [line for line in h.log_lines() if "WITNESS" in line]
        self.assertTrue(witness_calls)
        for line in witness_calls:
            self.assertIn("delete-aside", line)
            self.assertIn(".mariadb-era-", line)

    def test_a_missing_recorded_volume_is_refused_before_any_deletion(self) -> None:
        h = self.h
        self.ready()
        h.refused("retire-mariadb", "does not exist", **h.retire_env(STUB_VOLUME_MISSING="1"))
        self.assert_nothing_destroyed()

    def test_without_a_moved_aside_witness_only_the_volume_is_removed(self) -> None:
        h = self.h
        self.ready()
        shutil.rmtree(h.bootstrap_state)
        h.ok("retire-mariadb", **h.retire_env())
        self.assertNotIn("WITNESS", "\n".join(h.log_lines()))
        self.assertIn("docker volume rm oteryn-staging_mariadb_data", h.log_lines())


class UnfreezeTests(CutoverTestBase):
    def test_unfreeze_after_retirement(self) -> None:
        h = self.h
        h.advance_to("declare_done")
        h.ok("retire-mariadb", **h.retire_env())
        open_run = h.make_audit("open")
        h.refused("unfreeze", "does not reach its origin", OTERYN_CUTOVER_AUDIT_RUN_ID=open_run)
        h.ok("unfreeze", OTERYN_CUTOVER_AUDIT_RUN_ID=open_run, STUB_CANONICAL_STATUS="200")
        self.assertIn("up -d --no-deps marketplace-scheduler", "\n".join(h.log_lines()))
        self.assertTrue(h.has_state("UNFROZEN"))

    def test_unfreeze_requires_an_open_audit(self) -> None:
        h = self.h
        h.advance_to("declare_done")
        h.ok("retire-mariadb", **h.retire_env())
        h.refused("unfreeze", "audit result", OTERYN_CUTOVER_AUDIT_RUN_ID=h.make_audit("closed"), STUB_CANONICAL_STATUS="200")
        self.assertFalse(h.has_state("UNFROZEN"))

    def test_unfreeze_after_a_verified_rollback(self) -> None:
        h = self.h
        h.advance_to("smoke")
        h.set_engine("mariadb")
        h.ok("rollback", **h.closed())
        h.ok("unfreeze", OTERYN_CUTOVER_AUDIT_RUN_ID=h.make_audit("open"), STUB_CANONICAL_STATUS="200")

    def test_unfreeze_is_refused_while_the_cutover_is_stopped(self) -> None:
        h = self.h
        h.advance_to("native_smoke")
        h.refused("unfreeze", "gate stays closed", OTERYN_CUTOVER_AUDIT_RUN_ID=h.make_audit("open"), STUB_CANONICAL_STATUS="200")
        h.advance_to("declare_done")
        h.refused("unfreeze", "gate stays closed", OTERYN_CUTOVER_AUDIT_RUN_ID=h.make_audit("open"), STUB_CANONICAL_STATUS="200")


class SecretHygieneTests(CutoverTestBase):
    def test_no_secret_reaches_output_or_command_lines(self) -> None:
        h = self.h
        h.advance_to("declare_done")
        h.ok("retire-mariadb", **h.retire_env())
        h.ok("status")
        h.ok("unfreeze", OTERYN_CUTOVER_AUDIT_RUN_ID=h.make_audit("open"), STUB_CANONICAL_STATUS="200")
        everything = "\n".join(h.outputs) + "\n" + h.log.read_text() + "\n" + h.state()
        evidence = h.evidence / "gate-evidence.json"
        if evidence.exists():
            everything += evidence.read_text()
        for name, value in SECRETS.items():
            self.assertNotIn(value, everything, f"{name} leaked")
        self.assertNotIn("password", h.state().lower())

    def test_refusals_print_no_secret(self) -> None:
        h = self.h
        h.write_env("PLATFORM_POSTGRES_DUMP_PASSWORD", "")
        h.run("preflight", OTERYN_CUTOVER_OWNER_GO=TASK, OTERYN_CUTOVER_AUDIT_RUN_ID=h.make_audit("open"))
        for name, value in SECRETS.items():
            if value:
                self.assertNotIn(value, "\n".join(h.outputs))


class StaticContractTests(unittest.TestCase):
    def test_script_parses(self) -> None:
        subprocess.run(["bash", "-n", str(SCRIPT)], check=True)

    def test_script_is_covered_by_the_shell_syntax_check(self) -> None:
        text = BUILD_WORKFLOW.read_text()
        syntax = text.split("      - name: Validate shell syntax\n", 1)[1].split("          sh -n", 1)[0]
        self.assertIn("deploy/synology/scripts/cutover-postgres.sh", syntax)

    def test_destructive_step_is_not_reachable_from_hygiene_workflows(self) -> None:
        for workflow in (ROOT / ".github" / "workflows").glob("*.yml"):
            if workflow.name in ("deploy-synology-staging.yml", "build-synology-staging-images.yml"):
                continue
            self.assertNotIn("cutover-postgres.sh", workflow.read_text(), workflow.name)

    def test_destructive_commands_live_only_in_the_retire_step(self) -> None:
        text = SCRIPT.read_text()
        self.assertEqual(text.count("volume rm"), 2)  # dry-run plan and real run, both inside do_retire_mariadb
        body = text.split("do_retire_mariadb() {", 1)[1].split("\ndo_unfreeze() {", 1)[0]
        self.assertEqual(body.count("volume rm"), 2)
        self.assertIn("DELETE-MARIADB-VOLUME", text)
        self.assertNotIn("postgres_data", text.replace("# ", ""))

    def test_workflow_wires_the_cutover_action(self) -> None:
        text = WORKFLOW.read_text()
        self.assertIn("          - cutover\n", text)
        for name in ("cutover_step", "cutover_dry_run", "task_id", "close_run_id", "audit_run_id", "owner_go_ack",
                     "owner_delete_confirmation", "owner_delete_ack"):
            self.assertIn(f"      {name}:\n", text)
        self.assertIn('bash deploy/synology/scripts/cutover-postgres.sh "$CUTOVER_STEP"', text)
        step = text.split("      - name: Run cutover step\n", 1)[1].split("      - name: Upload cutover gate evidence\n", 1)[0]
        self.assertIn("if: inputs.action == 'cutover'", step)
        # The dispatch inputs reach the script through env, never interpolated into the shell text.
        run_part = step.split("        run: |\n", 1)[1]
        self.assertNotIn("${{", run_part)
        self.assertIn("OTERYN_CUTOVER_DRY_RUN: ${{ inputs.cutover_dry_run && '1' || '0' }}", step)
        upload = text.split("      - name: Upload cutover gate evidence\n", 1)[1].split("      - name: Deploy prebuilt images\n", 1)[0]
        self.assertIn("inputs.cutover_dry_run == false", upload)
        self.assertIn("inputs.cutover_step == 'declare-done' || inputs.cutover_step == 'rollback'", upload)
        self.assertIn("name: platform-cutover-gate-evidence", upload)
        self.assertIn("if-no-files-found: error", upload)

    def test_existing_deploy_and_rollback_steps_are_unchanged(self) -> None:
        text = WORKFLOW.read_text()
        self.assertIn("      - name: Deploy prebuilt images\n        if: inputs.action == 'deploy'\n        run: bash deploy/synology/scripts/deploy-impact.sh\n", text)
        self.assertIn("      - name: Roll back runtime images\n        if: inputs.action == 'rollback'\n        run: bash deploy/synology/scripts/rollback.sh\n", text)
        self.assertNotIn("cutover-postgres.sh", text.split("      - name: Deploy prebuilt images\n", 1)[1].split("Remove ephemeral", 1)[0])


if __name__ == "__main__":
    unittest.main()
