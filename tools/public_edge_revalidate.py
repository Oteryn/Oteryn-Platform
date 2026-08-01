#!/usr/bin/env python3
"""Bounded, read-only public edge observation for the two canonical Oteryn hosts."""

from __future__ import annotations

import hashlib
import json
import re
import socket
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

HOSTS = {
    "www": "oteryn.molehill.cloud",
    "gateway": "login.oteryn.molehill.cloud",
}
USER_AGENT = "Oteryn-Public-Edge-Revalidation/1.0"
OUT = Path("public-edge-revalidation")
OUT.mkdir(exist_ok=True)


def run(command: list[str], timeout: int = 30) -> dict[str, Any]:
    try:
        completed = subprocess.run(
            command,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        return {
            "returncode": completed.returncode,
            "stdout": completed.stdout[-12000:],
            "stderr": completed.stderr[-12000:],
        }
    except subprocess.TimeoutExpired as exc:
        return {
            "returncode": 124,
            "stdout": (exc.stdout or "")[-12000:] if isinstance(exc.stdout, str) else "",
            "stderr": "timeout",
        }


def dns_observation(host: str) -> dict[str, Any]:
    addresses: set[str] = set()
    error = None
    try:
        for item in socket.getaddrinfo(host, 443, type=socket.SOCK_STREAM):
            addresses.add(item[4][0])
    except OSError as exc:
        error = f"{type(exc).__name__}: {exc}"
    return {"addresses": sorted(addresses), "error": error}


def tls_observation(host: str, version: str) -> dict[str, Any]:
    flag = "-tls1_2" if version == "TLSv1.2" else "-tls1_3"
    result = run(
        [
            "openssl",
            "s_client",
            "-connect",
            f"{host}:443",
            "-servername",
            host,
            "-verify_hostname",
            host,
            flag,
            "-brief",
        ],
        timeout=25,
    )
    combined = (result["stdout"] + "\n" + result["stderr"]).strip()
    return {
        "ok": result["returncode"] == 0 and "Verification: OK" in combined,
        "returncode": result["returncode"],
        "summary": combined[-4000:],
    }


def certificate_observation(host: str) -> dict[str, Any]:
    shell = (
        f"timeout 25 openssl s_client -connect {host}:443 -servername {host} -showcerts </dev/null 2>/dev/null "
        "| openssl x509 -noout -subject -issuer -dates -ext subjectAltName -fingerprint -sha256"
    )
    result = run(["bash", "-lc", shell], timeout=30)
    return {
        "ok": result["returncode"] == 0 and "subject=" in result["stdout"].lower(),
        "returncode": result["returncode"],
        "summary": result["stdout"][-6000:] or result["stderr"][-2000:],
    }


def parse_headers(raw: str) -> dict[str, str]:
    blocks = re.split(r"\r?\n\r?\n", raw.strip()) if raw.strip() else []
    block = blocks[-1] if blocks else ""
    headers: dict[str, str] = {}
    for line in block.splitlines()[1:]:
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        headers[key.strip().lower()] = value.strip()
    return headers


def http_observation(
    url: str,
    *,
    method: str = "GET",
    body: str | None = None,
    accept: str = "*/*",
) -> dict[str, Any]:
    safe = hashlib.sha256(f"{method}:{url}".encode()).hexdigest()[:16]
    header_path = OUT / f"headers-{safe}.txt"
    body_path = OUT / f"body-{safe}.bin"
    command = [
        "curl",
        "-sS",
        "--max-time",
        "25",
        "--connect-timeout",
        "10",
        "--user-agent",
        USER_AGENT,
        "-H",
        f"Accept: {accept}",
        "-D",
        str(header_path),
        "-o",
        str(body_path),
        "-w",
        "%{http_code}\n%{url_effective}\n%{redirect_url}\n%{ssl_verify_result}\n",
        "-X",
        method,
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
        "body_bytes_sampled": len(payload),
        "body_sha256_sample": hashlib.sha256(payload).hexdigest(),
        "html_title": title,
        "body_prefix": re.sub(r"\s+", " ", text[:320]).strip(),
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
            "dns": dns_observation(host),
            "tls_1_2": tls_observation(host, "TLSv1.2"),
            "tls_1_3": tls_observation(host, "TLSv1.3"),
            "certificate": certificate_observation(host),
        }

    www_https = [
        "/",
        "/login?locale=en",
        "/register",
        "/forgot-password",
        "/health",
        "/news",
        "/highscores",
        "/version",
    ]
    gateway_https = ["/health", "/ready", "/version", "/login"]

    for path in www_https:
        key = f"www_https_{path}"
        evidence["requests"][key] = http_observation(
            f"https://{HOSTS['www']}{path}", accept="text/html,application/json"
        )
    for path in gateway_https:
        key = f"gateway_https_{path}"
        evidence["requests"][key] = http_observation(
            f"https://{HOSTS['gateway']}{path}", accept="application/json,text/html"
        )

    evidence["requests"]["gateway_invalid_login"] = http_observation(
        f"https://{HOSTS['gateway']}/v1/login",
        method="POST",
        body="{}",
        accept="application/json",
    )
    evidence["requests"]["www_http_root"] = http_observation(
        f"http://{HOSTS['www']}/", accept="text/html"
    )
    evidence["requests"]["gateway_http_health"] = http_observation(
        f"http://{HOSTS['gateway']}/health", accept="application/json,text/html"
    )

    (OUT / "evidence.json").write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")

    lines = [
        "# Oteryn public edge revalidation",
        "",
        f"Observed at: `{observed_at}`",
        "",
        "## DNS and TLS",
        "",
    ]
    for role, host_data in evidence["hosts"].items():
        lines.extend(
            [
                f"### {role}: `{host_data['hostname']}`",
                f"- addresses: `{', '.join(host_data['dns']['addresses']) or 'NONE'}`",
                f"- TLS 1.2 verified: `{host_data['tls_1_2']['ok']}`",
                f"- TLS 1.3 verified: `{host_data['tls_1_3']['ok']}`",
                f"- certificate extracted: `{host_data['certificate']['ok']}`",
                "",
            ]
        )
    lines.extend(["## HTTP observations", ""])
    for name, item in evidence["requests"].items():
        hsts = item["headers"].get("strict-transport-security", "")
        cache = item["headers"].get("cache-control", "")
        lines.append(
            f"- `{name}`: status `{item['http_code']}`, redirect `{item['redirect_url'] or ''}`, "
            f"server `{item['headers'].get('server', '')}`, HSTS `{hsts}`, cache `{cache}`, title `{item['html_title'] or ''}`"
        )
    lines.extend(
        [
            "",
            "This is a public, read-only observation. It does not prove production identity and performs no credentialed action.",
            "",
        ]
    )
    (OUT / "summary.md").write_text("\n".join(lines))
    print((OUT / "summary.md").read_text())


if __name__ == "__main__":
    main()
