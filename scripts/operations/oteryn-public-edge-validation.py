#!/usr/bin/env python3
"""Bounded read-only validation of the canonical Oteryn public endpoints."""
from __future__ import annotations

import hashlib
import json
import os
import re
import socket
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

WWW = "oteryn.molehill.cloud"
GATEWAY = "gateway.molehill.cloud"
HOSTS = {"www": WWW, "gateway": GATEWAY}
MACHINE_UA = "Oteryn-Public-Edge-Validation/2.0"
BROWSER_UA = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36"
)
OUT = Path(os.getenv("OTERYN_PUBLIC_EDGE_OUT", "public-edge-validation"))
SAFE_HEADERS = {
    "server",
    "location",
    "content-type",
    "cache-control",
    "pragma",
    "expires",
    "strict-transport-security",
    "cf-ray",
}


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
            "stderr": completed.stderr[-4000:],
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
    return {"address_count": len(addresses), "addresses": sorted(addresses), "error": error}


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
    verification_ok = "Verification: OK" in combined or "Verification: OK" in combined.replace("  ", " ")
    return {
        "ok": result["returncode"] == 0 and verification_ok,
        "returncode": result["returncode"],
        "protocol_seen": version in combined,
        "error_class": None if result["returncode"] == 0 else classify_error(combined),
    }


def certificate_observation(host: str) -> dict[str, Any]:
    shell = (
        f"timeout 25 openssl s_client -connect {host}:443 -servername {host} "
        "-showcerts </dev/null 2>/dev/null | "
        "openssl x509 -noout -subject -issuer -dates -fingerprint -sha256"
    )
    result = run(["bash", "-lc", shell], timeout=30)
    text = result["stdout"]
    fingerprint = None
    match = re.search(r"SHA256 Fingerprint=([^\r\n]+)", text, flags=re.I)
    if match:
        fingerprint = match.group(1).strip()
    return {
        "ok": result["returncode"] == 0 and "subject=" in text.lower(),
        "returncode": result["returncode"],
        "fingerprint_sha256": fingerprint,
        "metadata_sha256": hashlib.sha256(text.encode()).hexdigest() if text else None,
    }


def classify_error(text: str) -> str:
    normalized = text.lower()
    for marker, label in (
        ("certificate verify failed", "certificate_verify_failed"),
        ("hostname mismatch", "hostname_mismatch"),
        ("connection refused", "connection_refused"),
        ("temporary failure in name resolution", "dns_failure"),
        ("no peer certificate", "no_peer_certificate"),
        ("timeout", "timeout"),
    ):
        if marker in normalized:
            return label
    return "other"


def parse_headers(raw: str) -> dict[str, str]:
    blocks = re.split(r"\r?\n\r?\n", raw.strip()) if raw.strip() else []
    block = blocks[-1] if blocks else ""
    headers: dict[str, str] = {}
    for line in block.splitlines()[1:]:
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        normalized = key.strip().lower()
        if normalized in SAFE_HEADERS:
            headers[normalized] = value.strip()[:1000]
    return headers


def body_signals(payload: bytes) -> dict[str, Any]:
    text = payload.decode("utf-8", errors="replace")
    normalized = re.sub(r"\s+", " ", text).lower()
    title_match = re.search(r"<title[^>]*>(.*?)</title>", text, flags=re.I | re.S)
    title = re.sub(r"\s+", " ", title_match.group(1)).strip()[:200] if title_match else None
    return {
        "sampled_bytes": len(payload),
        "sample_sha256": hashlib.sha256(payload).hexdigest(),
        "html_title": title,
        "contains_cloudflare_interstitial": (
            "just a moment" in normalized
            or "attention required" in normalized
            or "cf-chl-" in normalized
        ),
        "contains_gateway_identity": "oteryn-game-gateway" in normalized,
        "contains_invalid_request": "invalid_request" in normalized,
        "contains_html_form": "<form" in normalized,
        "contains_platform_brand": "oteryn platform" in normalized,
    }


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
    return {
        "returncode": result["returncode"],
        "http_code": int(meta[0]) if meta and meta[0].isdigit() else 0,
        "effective_url": meta[1][:1000] if len(meta) > 1 else None,
        "redirect_url": meta[2][:1000] if len(meta) > 2 else None,
        "ssl_verify_result": meta[3] if len(meta) > 3 else None,
        "headers": parse_headers(raw_headers),
        "body": body_signals(payload),
        "error_class": None if result["returncode"] == 0 else classify_error(result["stderr"]),
    }


def private_no_store(item: dict[str, Any]) -> bool:
    cache = {
        token.strip().lower()
        for token in item.get("headers", {}).get("cache-control", "").split(",")
    }
    return {"private", "no-store", "no-cache"}.issubset(cache)


def positive_hsts(item: dict[str, Any]) -> bool:
    value = item.get("headers", {}).get("strict-transport-security", "")
    match = re.search(r"(?:^|;)\s*max-age=(\d+)", value, flags=re.I)
    return bool(match and int(match.group(1)) > 0)


def classify(evidence: dict[str, Any]) -> dict[str, Any]:
    requests = evidence["requests"]
    hosts = evidence["hosts"]
    acceptance: dict[str, bool] = {}
    acceptance["dns_www"] = hosts["www"]["dns"]["address_count"] > 0
    acceptance["dns_gateway"] = hosts["gateway"]["dns"]["address_count"] > 0
    acceptance["tls_www"] = hosts["www"]["tls_1_2"]["ok"] or hosts["www"]["tls_1_3"]["ok"]
    acceptance["tls_gateway"] = hosts["gateway"]["tls_1_2"]["ok"] or hosts["gateway"]["tls_1_3"]["ok"]
    browser_items = [value for key, value in requests.items() if key.startswith("www_browser_")]
    acceptance["www_browser_public"] = bool(browser_items) and all(
        200 <= item["http_code"] < 400
        and not item["body"]["contains_cloudflare_interstitial"]
        for item in browser_items
    )
    acceptance["gateway_health"] = requests["gateway_health"]["http_code"] == 200
    acceptance["gateway_ready"] = requests["gateway_ready"]["http_code"] == 200
    acceptance["gateway_version"] = (
        requests["gateway_version"]["http_code"] == 200
        and requests["gateway_version"]["body"]["contains_gateway_identity"]
    )
    invalid = requests["gateway_invalid_login"]
    acceptance["gateway_invalid_login"] = (
        invalid["http_code"] == 400
        and invalid["body"]["contains_invalid_request"]
        and private_no_store(invalid)
    )
    negative = requests["gateway_negative_login"]
    acceptance["gateway_no_www_cross_route"] = (
        negative["http_code"] == 404
        and not negative["body"]["contains_html_form"]
        and not negative["body"]["contains_platform_brand"]
    )
    acceptance["www_no_gateway_cross_route"] = not requests["www_version"]["body"]["contains_gateway_identity"]
    www_http = requests["www_http_root"]
    gateway_http = requests["gateway_http_health"]
    acceptance["http_redirect_www"] = (
        www_http["http_code"] in {301, 302, 307, 308}
        and (www_http.get("redirect_url") or "").startswith(f"https://{WWW}")
    )
    acceptance["http_redirect_gateway"] = (
        gateway_http["http_code"] in {301, 302, 307, 308}
        and (gateway_http.get("redirect_url") or "").startswith(f"https://{GATEWAY}")
    )
    acceptance["positive_hsts_www"] = positive_hsts(requests["www_browser_root"])

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
    failed = [name for name in required if not acceptance[name]]
    return {
        "acceptance": acceptance,
        "required_checks": required,
        "failed_required_checks": failed,
        "verdict": "PASS" if not failed else "FAIL",
    }


def live_evidence() -> dict[str, Any]:
    OUT.mkdir(parents=True, exist_ok=True)
    evidence: dict[str, Any] = {
        "schema_version": 2,
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "classification": "READ_ONLY_PUBLIC_EDGE_VALIDATION",
        "canonical_hosts": [WWW, GATEWAY],
        "hosts": {},
        "requests": {},
        "mutation": "none",
    }
    for role, host in HOSTS.items():
        evidence["hosts"][role] = {
            "hostname": host,
            "dns": dns_observation(host),
            "tls_1_2": tls_observation(host, "TLSv1.2"),
            "tls_1_3": tls_observation(host, "TLSv1.3"),
            "certificate": certificate_observation(host),
        }

    for name, path in (
        ("root", "/"),
        ("login", "/login?locale=en"),
        ("register", "/register"),
        ("forgot", "/forgot-password"),
        ("news", "/news"),
        ("highscores", "/highscores"),
    ):
        evidence["requests"][f"www_browser_{name}"] = http_observation(
            f"https://{WWW}{path}", user_agent=BROWSER_UA, accept="text/html,application/json"
        )
    evidence["requests"]["gateway_health"] = http_observation(
        f"https://{GATEWAY}/health", user_agent=MACHINE_UA, accept="application/json"
    )
    evidence["requests"]["gateway_ready"] = http_observation(
        f"https://{GATEWAY}/ready", user_agent=MACHINE_UA, accept="application/json"
    )
    evidence["requests"]["gateway_version"] = http_observation(
        f"https://{GATEWAY}/version", user_agent=MACHINE_UA, accept="application/json"
    )
    evidence["requests"]["gateway_negative_login"] = http_observation(
        f"https://{GATEWAY}/login", user_agent=MACHINE_UA, accept="text/html,application/json"
    )
    evidence["requests"]["gateway_invalid_login"] = http_observation(
        f"https://{GATEWAY}/v1/login",
        user_agent=MACHINE_UA,
        method="POST",
        body="{}",
        accept="application/json",
    )
    evidence["requests"]["www_version"] = http_observation(
        f"https://{WWW}/version", user_agent=MACHINE_UA, accept="application/json,text/html"
    )
    evidence["requests"]["www_http_root"] = http_observation(
        f"http://{WWW}/", user_agent=MACHINE_UA, accept="text/html"
    )
    evidence["requests"]["gateway_http_health"] = http_observation(
        f"http://{GATEWAY}/health", user_agent=MACHINE_UA, accept="application/json,text/html"
    )
    evidence.update(classify(evidence))
    return evidence


def main() -> None:
    fixture = os.getenv("OTERYN_PUBLIC_EDGE_FIXTURE")
    if fixture:
        evidence = json.loads(Path(fixture).read_text(encoding="utf-8"))
        evidence.update(classify(evidence))
        evidence.setdefault("mutation", "none")
    else:
        evidence = live_evidence()
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "evidence.json").write_text(
        json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    summary = [
        "# Oteryn public edge validation",
        "",
        f"Observed at: `{evidence.get('observed_at_utc', 'fixture')}`",
        f"Verdict: **{evidence['verdict']}**",
        f"Failed required checks: `{','.join(evidence['failed_required_checks']) or 'none'}`",
        "",
        "This observation is read-only and stores only bounded public-response metadata.",
        "",
    ]
    text = "\n".join(summary)
    (OUT / "summary.md").write_text(text, encoding="utf-8")
    print(text)
    if os.getenv("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(text)


if __name__ == "__main__":
    main()
