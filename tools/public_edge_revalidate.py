#!/usr/bin/env python3
"""Read-only post-Cloudflare public edge observation for Oteryn."""

from __future__ import annotations

import hashlib
import json
import re
import socket
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

OUT = Path("public-edge-revalidation")
OUT.mkdir(exist_ok=True)
HOSTS = {
    "www": "oteryn.molehill.cloud",
    "gateway": "login.oteryn.molehill.cloud",
}
UA = "Oteryn-Public-Edge-Revalidation/2.0"


def run(command: list[str], timeout: int = 30) -> dict[str, Any]:
    try:
        completed = subprocess.run(command, text=True, capture_output=True, timeout=timeout, check=False)
        return {
            "returncode": completed.returncode,
            "stdout": completed.stdout[-12000:],
            "stderr": completed.stderr[-12000:],
        }
    except subprocess.TimeoutExpired:
        return {"returncode": 124, "stdout": "", "stderr": "timeout"}


def dns(host: str) -> dict[str, Any]:
    addresses: set[str] = set()
    error = None
    try:
        for item in socket.getaddrinfo(host, 443, type=socket.SOCK_STREAM):
            addresses.add(item[4][0])
    except OSError as exc:
        error = f"{type(exc).__name__}: {exc}"
    return {"addresses": sorted(addresses), "error": error}


def tls(host: str, version: str) -> dict[str, Any]:
    flag = "-tls1_2" if version == "1.2" else "-tls1_3"
    result = run([
        "openssl", "s_client", "-connect", f"{host}:443", "-servername", host,
        "-verify_hostname", host, flag, "-brief",
    ], timeout=25)
    combined = (result["stdout"] + "\n" + result["stderr"]).strip()
    return {
        "ok": result["returncode"] == 0 and "Verification: OK" in combined,
        "returncode": result["returncode"],
        "summary": combined[-4000:],
    }


def certificate(host: str) -> dict[str, Any]:
    shell = (
        f"timeout 25 openssl s_client -connect {host}:443 -servername {host} -showcerts </dev/null 2>/dev/null "
        "| openssl x509 -noout -subject -issuer -dates -ext subjectAltName -fingerprint -sha256"
    )
    result = run(["bash", "-lc", shell], timeout=30)
    text = result["stdout"][-6000:] or result["stderr"][-2000:]
    return {
        "ok": result["returncode"] == 0 and "subject=" in text.lower(),
        "returncode": result["returncode"],
        "summary": text,
    }


def parse_headers(raw: str) -> dict[str, str]:
    blocks = re.split(r"\r?\n\r?\n", raw.strip()) if raw.strip() else []
    block = blocks[-1] if blocks else ""
    headers: dict[str, str] = {}
    for line in block.splitlines()[1:]:
        if ":" in line:
            key, value = line.split(":", 1)
            headers[key.strip().lower()] = value.strip()
    return headers


def request(name: str, url: str, method: str = "GET", body: str | None = None) -> dict[str, Any]:
    tag = hashlib.sha256(name.encode()).hexdigest()[:16]
    header_path = OUT / f"headers-{tag}.txt"
    body_path = OUT / f"body-{tag}.bin"
    command = [
        "curl", "-sS", "--max-time", "25", "--connect-timeout", "10",
        "--user-agent", UA, "-H", "Accept: text/html,application/json",
        "-D", str(header_path), "-o", str(body_path),
        "-w", "%{http_code}\n%{url_effective}\n%{redirect_url}\n%{ssl_verify_result}\n",
        "-X", method,
    ]
    if body is not None:
        command.extend(["-H", "Content-Type: application/json", "--data-binary", body])
    command.append(url)
    result = run(command, timeout=35)
    meta = result["stdout"].splitlines()
    raw_headers = header_path.read_text(errors="replace") if header_path.exists() else ""
    payload = body_path.read_bytes()[:8192] if body_path.exists() else b""
    text = payload.decode("utf-8", errors="replace")
    title_match = re.search(r"<title[^>]*>(.*?)</title>", text, flags=re.I | re.S)
    title = re.sub(r"\s+", " ", title_match.group(1)).strip()[:240] if title_match else None
    return {
        "returncode": result["returncode"],
        "http_code": int(meta[0]) if meta and meta[0].isdigit() else 0,
        "effective_url": meta[1] if len(meta) > 1 else None,
        "redirect_url": meta[2] if len(meta) > 2 else None,
        "ssl_verify_result": meta[3] if len(meta) > 3 else None,
        "headers": parse_headers(raw_headers),
        "html_title": title,
        "body_prefix": re.sub(r"\s+", " ", text[:400]).strip(),
        "body_sha256_sample": hashlib.sha256(payload).hexdigest(),
        "error": result["stderr"][-2000:] or None,
    }


def main() -> None:
    observed_at = datetime.now(timezone.utc).isoformat()
    evidence: dict[str, Any] = {
        "observed_at_utc": observed_at,
        "runner_observation_only": True,
        "hosts": {},
        "requests": {},
    }

    for role, host in HOSTS.items():
        evidence["hosts"][role] = {
            "hostname": host,
            "dns": dns(host),
            "tls_1_2": tls(host, "1.2"),
            "tls_1_3": tls(host, "1.3"),
            "certificate": certificate(host),
        }

    for path in ["/", "/login?locale=en", "/register", "/forgot-password", "/health", "/news", "/highscores", "/version"]:
        evidence["requests"][f"www_https_{path}"] = request(
            f"www_https_{path}", f"https://{HOSTS['www']}{path}"
        )
    for path in ["/health", "/ready", "/version", "/login"]:
        evidence["requests"][f"gateway_https_{path}"] = request(
            f"gateway_https_{path}", f"https://{HOSTS['gateway']}{path}"
        )
    evidence["requests"]["gateway_invalid_login"] = request(
        "gateway_invalid_login", f"https://{HOSTS['gateway']}/v1/login", method="POST", body="{}"
    )
    evidence["requests"]["www_http_root"] = request(
        "www_http_root", f"http://{HOSTS['www']}/"
    )
    evidence["requests"]["gateway_http_health"] = request(
        "gateway_http_health", f"http://{HOSTS['gateway']}/health"
    )

    (OUT / "evidence.json").write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")

    lines = [
        "# Oteryn public edge post-Cloudflare revalidation",
        "",
        f"Observed at: `{observed_at}`",
        "",
        "## DNS and TLS",
        "",
    ]
    for role, item in evidence["hosts"].items():
        lines.extend([
            f"### {role}: `{item['hostname']}`",
            f"- addresses: `{', '.join(item['dns']['addresses']) or 'NONE'}`",
            f"- TLS 1.2 verified: `{item['tls_1_2']['ok']}`",
            f"- TLS 1.3 verified: `{item['tls_1_3']['ok']}`",
            f"- certificate extracted: `{item['certificate']['ok']}`",
            "",
        ])
    lines.extend(["## HTTP observations", ""])
    for name, item in evidence["requests"].items():
        headers = item["headers"]
        lines.append(
            f"- `{name}`: status `{item['http_code']}`, redirect `{item['redirect_url'] or ''}`, "
            f"server `{headers.get('server', '')}`, HSTS `{headers.get('strict-transport-security', '')}`, "
            f"cache `{headers.get('cache-control', '')}`, title `{item['html_title'] or ''}`"
        )
    lines.extend([
        "",
        "This is a public, read-only observation. It performs no credentialed or mutating action.",
        "",
    ])
    (OUT / "summary.md").write_text("\n".join(lines))
    print((OUT / "summary.md").read_text())


if __name__ == "__main__":
    main()
