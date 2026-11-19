"""Native gameplay shares :443 with the HTTP edge by SNI and ALPN.

The static checks always run. The end-to-end check runs when
OTERYN_EDGE_CADDY names a Caddy binary built with caddy-l4 (CI builds one
from the pins in edge.Dockerfile): it starts that Caddy with the production
listener wrappers and proves the relay is a raw TLS passthrough.
"""
import os
from pathlib import Path
import re
import socket
import ssl
import subprocess
import tempfile
import threading
import time
import unittest

HERE = Path(__file__).resolve().parent
CADDYFILE = HERE / 'Caddyfile'
GAME_HOST = 'game.oteryn.com'
GAME_ALPN = 'oteryn-game/1'
GAME_UPSTREAM = '172.18.0.11:7172'


def global_block(text):
    """The leading global options block of a Caddyfile, braces balanced."""
    assert text.startswith('{\n')
    depth = 0
    for index, char in enumerate(text):
        depth += {'{': 1, '}': -1}.get(char, 0)
        if depth == 0:
            return text[:index + 1]
    raise AssertionError('unbalanced global block')


class StaticContract(unittest.TestCase):
    def test_gameplay_route_matches_sni_and_alpn_before_tls_and_relays_raw(self):
        text = CADDYFILE.read_text()
        block = global_block(text)
        wrappers = block[block.index('listener_wrappers'):]
        self.assertLess(wrappers.index('layer4'), wrappers.index('\n\t\t\ttls\n'))
        self.assertIn('sni ' + GAME_HOST, wrappers)
        self.assertIn('alpn ' + GAME_ALPN, wrappers)
        self.assertIn('proxy ' + GAME_UPSTREAM, wrappers)
        # No termination and no PROXY header on the gameplay relay.
        self.assertNotRegex(wrappers, r'proxy_protocol|\btls\s*\{[^}]*\}\s*\n\s*proxy|handshake')
        # The HTTP site for the Game host stays only for its ACME certificate.
        site = text[text.index('\ngame.oteryn.com {'):]
        self.assertNotIn('reverse_proxy', site)

    def test_runtime_compose_keeps_the_game_listener_private_at_the_relay_address(self):
        compose = (HERE / 'game-runtime.compose.yml').read_text()
        game = compose.split('\n  ops:\n', 1)[0]
        self.assertNotIn('ports:', game)
        self.assertNotIn('7172:7172', compose)
        self.assertIn('ipv4_address: ' + GAME_UPSTREAM.split(':')[0], game)

    def test_edge_image_is_built_from_pinned_inputs(self):
        dockerfile = (HERE / 'edge.Dockerfile').read_text()
        froms = re.findall(r'(?m)^FROM (\S+)', dockerfile)
        self.assertEqual(len(froms), 2)
        for image in froms:
            self.assertRegex(image, r'^caddy:[^@]+@sha256:[0-9a-f]{64}$')
        self.assertRegex(dockerfile, r'xcaddy build v\d+\.\d+\.\d+ --with github\.com/mholt/caddy-l4@v\d+\.\d+\.\d+\n')
        compose = (HERE / 'edge.compose.yml').read_text()
        self.assertRegex(compose, r'(?m)^    image: oteryn-edge:2\.11\.7-l4-0\.1\.3$')
        # The rollback image is the stock base of the final stage.
        self.assertIn(froms[1], (HERE / 'README.md').read_text())


def free_port():
    with socket.socket() as probe:
        probe.bind(('127.0.0.1', 0))
        return probe.getsockname()[1]


def wait_listening(port, deadline=20.0):
    end = time.monotonic() + deadline
    while time.monotonic() < end:
        try:
            with socket.create_connection(('127.0.0.1', port), timeout=0.5):
                return
        except OSError:
            time.sleep(0.1)
    raise AssertionError(f'port {port} never listened')


@unittest.skipUnless(os.environ.get('OTERYN_EDGE_CADDY'), 'OTERYN_EDGE_CADDY is unset')
class Passthrough(unittest.TestCase):
    def setUp(self):
        self.directory = Path(tempfile.mkdtemp())
        self.cert, self.key = self.directory / 'game.crt', self.directory / 'game.key'
        subprocess.run(['openssl', 'req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256',
                        '-nodes', '-days', '1', '-subj', '/CN=' + GAME_HOST,
                        '-addext', 'subjectAltName=DNS:' + GAME_HOST,
                        '-keyout', str(self.key), '-out', str(self.cert)],
                       check=True, capture_output=True)
        self.seen_alpn = []
        upstream = free_port()
        server = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        server.minimum_version = ssl.TLSVersion.TLSv1_3
        server.set_alpn_protocols([GAME_ALPN])
        server.load_cert_chain(self.cert, self.key)
        self.listener = socket.create_server(('127.0.0.1', upstream))

        def serve():
            while True:
                try:
                    raw, _ = self.listener.accept()
                except OSError:
                    return
                try:
                    with server.wrap_socket(raw, server_side=True) as tls:
                        self.seen_alpn.append(tls.selected_alpn_protocol())
                        tls.sendall(b'GAME')
                except (OSError, ssl.SSLError):
                    pass

        threading.Thread(target=serve, daemon=True).start()
        self.https, http = free_port(), free_port()
        block = global_block(CADDYFILE.read_text()).replace(GAME_UPSTREAM, f'127.0.0.1:{upstream}')
        block = block.replace('{\n', '{\n\tadmin off\n\tskip_install_trust\n\tlocal_certs\n'
                              f'\thttp_port {http}\n\thttps_port {self.https}\n', 1)
        config = self.directory / 'Caddyfile'
        config.write_text(block + '\n\nsite.oteryn.test {\n\ttls internal\n\trespond "http-app"\n}\n')
        environment = dict(os.environ, XDG_DATA_HOME=str(self.directory / 'data'),
                           XDG_CONFIG_HOME=str(self.directory / 'config'))
        self.caddy = subprocess.Popen([os.environ['OTERYN_EDGE_CADDY'], 'run', '--config', str(config),
                                       '--adapter', 'caddyfile'], env=environment,
                                      stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        wait_listening(self.https)

    def tearDown(self):
        self.caddy.terminate()
        self.caddy.wait(timeout=10)
        self.caddy.stderr.close()
        self.listener.close()

    def connect(self, server_name, alpn, context):
        context.set_alpn_protocols(alpn)
        raw = socket.create_connection(('127.0.0.1', self.https), timeout=10)
        return context.wrap_socket(raw, server_hostname=server_name)

    def test_gameplay_is_relayed_without_termination_and_the_rest_reaches_http(self):
        # The client verifies the Game node's own certificate end to end and
        # negotiates TLS 1.3 with the Game ALPN: the edge did not terminate.
        game = ssl.create_default_context(cafile=str(self.cert))
        game.minimum_version = ssl.TLSVersion.TLSv1_3
        for _ in range(3):
            with self.connect(GAME_HOST, [GAME_ALPN], game) as tls:
                self.assertEqual(tls.version(), 'TLSv1.3')
                self.assertEqual(tls.selected_alpn_protocol(), GAME_ALPN)
                self.assertEqual(tls.recv(4), b'GAME')
        self.assertEqual(self.seen_alpn, [GAME_ALPN] * 3)
        # Any other connection continues to the HTTP app.
        insecure = ssl.create_default_context()
        insecure.check_hostname = False
        insecure.verify_mode = ssl.CERT_NONE
        with self.connect('site.oteryn.test', ['http/1.1'], insecure) as tls:
            tls.sendall(b'GET / HTTP/1.1\r\nHost: site.oteryn.test\r\nConnection: close\r\n\r\n')
            body = b''
            while chunk := tls.recv(4096):
                body += chunk
        self.assertIn(b'http-app', body)
        # The Game host without the Game ALPN is not relayed.
        relayed = len(self.seen_alpn)
        with self.assertRaises((ssl.SSLError, OSError)):
            with self.connect(GAME_HOST, ['h2', 'http/1.1'], game) as tls:
                tls.recv(4)
        self.assertEqual(len(self.seen_alpn), relayed)


if __name__ == '__main__':
    unittest.main()
