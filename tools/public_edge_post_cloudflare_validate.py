#!/usr/bin/env python3
"""Bounded, read-only validation of Oteryn public domains after Cloudflare apply."""

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
MACHINE_UA = "Oteryn-Public-Edge-Post-Cloudflare/1.0"
BROWSER_UA = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36"
)
OUT = Path("public-edge-post-cloudflare")
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
        f"timeout 25 openssl s_client -connect {host}:443 -servername {host} "
        "-showcerts </dev/null 2>/dev/null | openssl x509 -noout -subject "
        "-issuer -dates -ext subjectAltName -fingerprint -sha256"
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
    user_agent: str,
    method: str = "GET",
    body: str | None = None,
    accept: str = "*/*",
) -> dict[str, Any]:
    safe = hashlib.sha256(f"{method}:{user_agent}:{url}".encode()).hexdigest()[:16]
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
        user_agent,
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


def is_cloudflare_interstitial(item: dict[str, Any]) -> bool:
    title = (item.get("html_title") or "").lower()
    prefix = (item.get("body_prefix") or "").lower()
    return item.get("http_code") == 403 and (
        "just a moment" in title
        or "attention required" in title
        or "cloudflare" in prefix
    )


def has_positive_hsts(item: dict[str, Any]) -> bool:
    value = item.get("headers", {}).get("strict-transport-security", "")
    match = re.search(r"(?:^|;)\s*max-age=(\d+)", value, flags=re.I)
    return bool(match and int(match.group(1)) > 0)


def private_no_store(item: dict[str, Any]) -> bool:
    cache = {token.strip().lower() for token in item.get("headers", {}).get("cache-control", "").split(",")}
    return {"private", "no-store", "no-cache"}.issubset(cache)


def main() -> None:
    observed_at = datetime.now(timezone.utc).isoformat()
    evidence: dict[str, Any] = {
        "observed_at_utc": observed_at,
        "runner_observation_only": True,
        "cloudflare_apply_reference": "run 30700054602",
        "hosts": {},
        "requests": {},
        "acceptance": {},
    }

    for role, host in HOSTS.items():
        evidence["hosts"][role] = {
            "hostname": host,
            "dns": dns_observation(host),
            "tls_1_2": tls_observation(host, "TLSv1.2"),
            "tls_1_3": tls_observation(host, "TLSv1.3"),
            "certificate": certificate_observation(host),
        }

    www_paths = ["/", "/login?locale=en", "/register", "/forgot-password", "/news", "/highscores"]
    for path in www_paths:
        evidence["requests"][f"www_machine_{path}"] = http_observation(
            f"https://{HOSTS['www']}{path}",
            user_agent=MACHINE_UA,
            accept="text/html,application/json",
        )
        evidence["requests"][f"www_browser_{path}"] = http_observation(
            f"https://{HOSTS['www']}{path}",
            user_agent=BROWSER_UA,
            accept="text/html,application/json",
        )

    for path in ["/health", "/ready", "/version", "/login"]:
        evidence["requests"][f"gateway_https_{path}"] = http_observation(
            f"https://{HOSTS['gateway']}{path}",
            user_agent=MACHINE_UA,
            accept="application/json,text/html",
        )

    evidence["requests"]["gateway_invalid_login"] = http_observation(
        f"https://{HOSTS['gateway']}/v1/login",
        user_agent=MACHINE_UA,
        method="POST",
        body="{}",
        accept="application/json",
    )
    evidence["requests"]["www_cross_route_version"] = http_observation(
        f"https://{HOSTS['www']}/version",
        user_agent=MACHINE_UA,
        accept="application/json,text/html",
    )
    evidence["requests"]["www_http_root"] = http_observation(
        f"http://{HOSTS['www']}/",
        user_agent=MACHINE_UA,
        accept="text/html",
    )
    evidence["requests"]["gateway_http_health"] = http_observation(
        f"http://{HOSTS['gateway']}/health",
        user_agent=MACHINE_UA,
        accept="application/json,text/html",
    )

    requests = evidence["requests"]
    hosts = evidence["hosts"]
    acceptance = evidence["acceptance"]

    acceptance["dns_www"] = bool(hosts["www"]["dns"]["addresses"])
    acceptance["dns_gateway"] = bool(hosts["gateway"]["dns"]["addresses"])
    acceptance["tls_www"] = hosts["www"]["tls_1_2"]["ok"] or hosts["www"]["tls_1_3"]["ok"]
    acceptance["tls_gateway"] = hosts["gateway"]["tls_1_2"]["ok"] or hosts["gateway"]["tls_1_3"]["ok"]
    acceptance["www_browser_public"] = all(
        item["http_code"] not in {0, 403} and not is_cloudflare_interstitial(item)
        for key, item in requests.items()
        if key.startswith("www_browser_")
    )
    acceptance["gateway_health"] = requests["gateway_https_/health"]["http_code"] == 200
    acceptance["gateway_ready"] = requests["gateway_https_/ready"]["http_code"] == 200
    version = requests["gateway_https_/version"]
    acceptance["gateway_version"] = version["http_code"] == 200 and "oteryn-game-gateway" in version["body_prefix"]
    invalid = requests["gateway_invalid_login"]
    acceptance["gateway_invalid_login"] = (
        invalid["http_code"] == 400
        and "invalid_request" in invalid["body_prefix"]
        and private_no_store(invalid)
    )
    negative = requests["gateway_https_/login"]
    acceptance["gateway_no_www_cross_route"] = (
        negative["http_code"] == 404
        and "<form" not in negative["body_prefix"].lower()
        and "oteryn platform" not in negative["body_prefix"].lower()
    )
    acceptance["www_no_gateway_cross_route"] = "oteryn-game-gateway" not in requests["www_cross_route_version"]["body_prefix"]
    acceptance["http_redirect_www"] = requests["www_http_root"]["http_code"] in {301, 302, 307, 308} and (
        requests["www_http_root"]["redirect_url"] or ""
    ).startswith(f"https://{HOSTS['www']}")
    acceptance["http_redirect_gateway"] = requests["gateway_http_health"]["http_code"] in {301, 302, 307, 308} and (
        requests["gateway_http_health"]["redirect_url"] or ""
    ).startswith(f"https://{HOSTS['gateway']}")
    acceptance["positive_hsts_www"] = has_positive_hsts(requests["www_browser_/"])

    required = [
        "dns_www",
        "dns_gateway",
        "tls_www",
        "tls_gateway",
        "www_browser_public",
        "gateway_health",
        "gateway_ready",
        "gateway_version",
        "gateway_invalid_login",
        "gateway_no_www_cross_route",
        "www_no_gateway_cross_route",
        "http_redirect_www",
        "http_redirect_gateway",
    ]
    evidence["verdict"] = "PASS" if all(acceptance[name] for name in required) else "FAIL"
    evidence["failed_required_checks"] = [name for name in required if not acceptance[name]]

    (OUT / "evidence.json").write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")

    lines = [
        "# Oteryn public edge validation after Cloudflare apply",
        "",
        f"Observed at: `{observed_at}`",
        f"Verdict: **{evidence['verdict']}**",
        f"Failed required checks: `{', '.join(evidence['failed_required_checks']) or 'none'}`",
        "",
        "## DNS and TLS",
        "",
    ]
    for role, host_data in hosts.items():
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
    lines.extend(["## Acceptance", ""])
    for name, result in acceptance.items():
        lines.append(f"- `{name}`: `{result}`")
    lines.extend(["", "## HTTP observations", ""])
    for name, item in requests.items():
        hsts = item["headers"].get("strict-transport-security", "")
        cache = item["headers"].get("cache-control", "")
        lines.append(
            f"- `{name}`: status `{item['http_code']}`, redirect `{item['redirect_url'] or ''}`, "
            f"server `{item['headers'].get('server', '')}`, HSTS `{hsts}`, cache `{cache}`, title `{item['html_title'] or ''}`"
        )
    lines.extend(
        [
            "",
            "This is a public, read-only observation. It uses no credential, cookie, reset token, valid Game Login Ticket or state-changing application request.",
            "",
        ]
    )
    (OUT / "summary.md").write_text("\n".join(lines))
    print((OUT / "summary.md").read_text())


if __name__ == "__main__":
    main()
