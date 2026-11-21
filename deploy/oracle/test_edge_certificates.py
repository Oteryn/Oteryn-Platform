"""edge-certificates.py fails on short-lived, untrusted or wrong-name certificates."""
import datetime
import importlib.util
from pathlib import Path
import socket
import ssl
import subprocess
import tempfile
import threading
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('edge_certificates', HERE / 'edge-certificates.py')
edge = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(edge)
NAME = 'edge.test.invalid'


class TlsServer:
    """One-shot local TLS server presenting a self-signed certificate for NAME."""

    def __init__(self, directory, days):
        self.cert = Path(directory) / f'{days}.pem'
        key = Path(directory) / f'{days}.key'
        subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', str(days),
                        '-subj', '/CN=' + NAME, '-addext', 'subjectAltName=DNS:' + NAME,
                        '-keyout', str(key), '-out', str(self.cert)], check=True, capture_output=True)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(self.cert, key)
        self.listener = socket.create_server(('127.0.0.1', 0))
        self.port = self.listener.getsockname()[1]
        threading.Thread(target=self.serve, args=(context,), daemon=True).start()

    def serve(self, context):
        with self.listener:
            conn, _ = self.listener.accept()
        try:
            with context.wrap_socket(conn, server_side=True) as tls:
                tls.recv(1)
        except (OSError, ssl.SSLError):
            pass

    def trusting(self):
        return ssl.create_default_context(cafile=str(self.cert))


class EdgeCertificates(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.now = datetime.datetime.now(datetime.timezone.utc)

    def tearDown(self):
        self.tmp.cleanup()

    def run_check(self, days, host=NAME, trusted=True):
        server = TlsServer(self.tmp.name, days)
        context = server.trusting() if trusted else ssl.create_default_context()
        return edge.check(host, '127.0.0.1', context, self.now, port=server.port, timeout=5)

    def test_valid_certificate_with_enough_time_passes(self):
        ok, line = self.run_check(60)
        self.assertTrue(ok, line)
        self.assertIn('ok 6', line)

    def test_certificate_expiring_inside_the_threshold_fails(self):
        ok, line = self.run_check(edge.MIN_DAYS - 1)
        self.assertFalse(ok, line)
        self.assertIn('FAIL', line)

    def test_untrusted_certificate_fails(self):
        ok, line = self.run_check(60, trusted=False)
        self.assertFalse(ok, line)
        self.assertIn('SSLCertVerificationError', line)

    def test_certificate_for_another_name_fails(self):
        ok, line = self.run_check(60, host='other.test.invalid')
        self.assertFalse(ok, line)

    def test_every_public_edge_host_is_checked(self):
        self.assertEqual(set(edge.HOSTS), {'oteryn.com', 'www.oteryn.com', 'api.oteryn.com',
                                           'login.oteryn.com', 'gateway.oteryn.com', 'game.oteryn.com'})


if __name__ == '__main__':
    unittest.main()
