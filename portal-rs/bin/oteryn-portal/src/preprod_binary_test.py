"""Composed-binary regression: run after cargo build -p oteryn-portal.

Usage: python3 preprod_binary_test.py /path/to/oteryn-portal
Requires PORTAL_TEST_DATABASE_URL for the disposable local PostgreSQL service.
"""
import http.client
import os
from pathlib import Path
import signal
import socket
import ssl
import subprocess
import sys
import tempfile
import time
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit
import uuid


def port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def openssl(*args):
    subprocess.run(["openssl", *args], check=True, stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL)


def _preprod_binary_reaches_enabled_surfaces(binary, database_url):
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="fix07-tls-") as directory:
        root = Path(directory)
        ca, ca_key = root / "ca.pem", root / "ca.key"
        openssl("req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256",
                "-nodes", "-keyout", str(ca_key), "-out", str(ca), "-days", "1",
                "-subj", "/CN=fix07-test-ca", "-addext", "basicConstraints=critical,CA:TRUE")

        def issue(name, usage):
            cert, key, csr, ext = [root / f"{name}.{suffix}" for suffix in ("pem", "key", "csr", "ext")]
            openssl("req", "-new", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256",
                    "-nodes", "-keyout", str(key), "-out", str(csr), "-subj", f"/CN={name}")
            ext.write_text(f"subjectAltName=DNS:{name}\nextendedKeyUsage={usage}\n")
            openssl("x509", "-req", "-in", str(csr), "-CA", str(ca), "-CAkey", str(ca_key),
                    "-CAcreateserial", "-out", str(cert), "-days", "1", "-extfile", str(ext))
            return cert, key

        cert, key = issue("localhost", "serverAuth")
        public_port, private_port = port(), port()
        env = {k: v for k, v in os.environ.items()
               if not k.startswith(("OTERYN_", "PORTAL_", "GAME_AUTH_", "PRODUCTS_ENTITLEMENTS_"))}
        env.update({
            "OTERYN_DATABASE_URL": database_url,
            "OTERYN_ENV": "preprod", "APP_ENV": "preproduction",
            "OTERYN_BIND": f"127.0.0.1:{public_port}",
            "OTERYN_INTERNAL_TLS_BIND": f"127.0.0.1:{private_port}",
            "OTERYN_INTERNAL_TLS_CERT_FILE": str(cert),
            "OTERYN_INTERNAL_TLS_KEY_FILE": str(key),
            "OTERYN_INTERNAL_TLS_CLIENT_CA_FILE": str(ca),
            "PORTAL_CHARACTER_COMMANDS_ENABLED": "true",
            "PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY": "oteryn:character-authority:primary",
            "PORTAL_CHARACTER_COMMAND_IDENTITIES": '{"commands.oteryn.internal":[]}',
            "PORTAL_CHARACTER_SERVICES_EXCHANGE_ENABLED": "true",
            "PORTAL_CHARACTER_SERVICES_PROJECTION_ENABLED": "true",
            "PORTAL_CHARACTER_SERVICES_PROJECTION_AUTHORITY": "oteryn:game:primary",
            "PORTAL_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS": "1",
            "PORTAL_CHARACTER_SERVICES_FRESHNESS_SECONDS": "30",
            "PORTAL_CHARACTER_SERVICES_IDENTITY": "services.oteryn.internal",
            "PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED": "true",
            "PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED": "true",
            "PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY": "oteryn:game:primary",
            "PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY": "store.oteryn.internal",
            "PORTAL_PLAYERDATA_G10_INGEST_ENABLED": "true",
            "PORTAL_PLAYERDATA_G10_SOURCE_AUTHORITY": "OTERYN_GAME_JOURNAL",
            "PORTAL_PLAYERDATA_JOURNAL_ITEMS_SAN": "items.oteryn.internal",
            "PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED": "true",
            "PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY": "oteryn:journal:primary",
            "PORTAL_PLAYERDATA_JOURNAL_PROGRESS_SAN": "progress.oteryn.internal",
            "PORTAL_PLAYERDATA_JOURNAL_FAMILIES": "BANK,ITEMS,stamina",
        })
        subprocess.run([binary, "migrate"], env=env, check=True, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL)
        process = subprocess.Popen([binary, "serve"], env=env, stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL)
        try:
            deadline = time.monotonic() + 15
            while True:
                assert process.poll() is None, "composed serve process exited during startup"
                try:
                    with socket.create_connection(("127.0.0.1", private_port), timeout=0.1):
                        break
                except OSError:
                    assert time.monotonic() < deadline, "private listener did not start"
                    time.sleep(0.05)
            surfaces = {
                "commands": [f"/internal/{version}/game-auth/character-commands/{action}"
                             for version in ("v1", "v2") for action in ("pending", "read", "receipt")],
                "services": [f"/internal/v2/game-auth/character-services/{action}" for action in ("state", "watermark")],
                "store": [f"/internal/v1/products-entitlements/store/{action}" for action in ("balance/read", "history/read", "purchases")],
                "items": [f"/internal/v1/playerdata/journal/{action}" for action in ("records", "cursor")],
                "progress": [f"/internal/v1/playerdata/progress/{action}" for action in ("record", "watermark")],
            }
            for peer, paths in surfaces.items():
                cert, key = issue(f"{peer}.oteryn.internal", "clientAuth")
                context = ssl.create_default_context(cafile=str(ca))
                context.minimum_version = ssl.TLSVersion.TLSv1_3
                context.set_alpn_protocols(["http/1.1"])
                context.load_cert_chain(cert, key)
                for path in paths:
                    connection = http.client.HTTPSConnection("localhost", private_port, context=context, timeout=5)
                    connection.request("POST", path, body=b"not json", headers={"Content-Type": "application/json"})
                    response = connection.getresponse()
                    assert response.status == 400, f"{path}: expected authenticated body refusal, got {response.status}"
                    response.read()
                    connection.close()
                    print(f"PASS preprod mTLS {path}")
        finally:
            process.send_signal(signal.SIGINT)
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
    print(f"PASS test_preprod_binary_reaches_enabled_surfaces duration={time.monotonic() - started:.3f}s")


def test_preprod_binary_reaches_enabled_surfaces(binary):
    schema = "fix07_" + uuid.uuid4().hex
    command = [os.environ.get("PORTAL_TEST_PSQL", "psql"), "-X", "-At", "-v", "ON_ERROR_STOP=1"]
    env = os.environ.copy()
    env["PGDATABASE"] = env["PORTAL_TEST_DATABASE_URL"]

    def sql(text):
        subprocess.run(command, input=text, env=env, text=True, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    sql(f"CREATE SCHEMA {schema};")
    try:
        url = urlsplit(env["PORTAL_TEST_DATABASE_URL"])
        options = dict(parse_qsl(url.query))
        options["options"] = f"-csearch_path={schema}"
        database_url = urlunsplit(url._replace(query=urlencode(options)))
        _preprod_binary_reaches_enabled_surfaces(binary, database_url)
    finally:
        sql(f"DROP SCHEMA {schema} CASCADE;")


if __name__ == "__main__":
    test_preprod_binary_reaches_enabled_surfaces(str(Path(sys.argv[1]).resolve()))
