#!/usr/bin/env python3
"""Fail when a certificate served by the Oracle edge is invalid or expires soon.

Connects to the origin address directly with each public host name as SNI, so
the apex reports Caddy's own certificate (the one renewed through the tunnel),
not Cloudflare's. The TLS handshake completes before the apex's tunnel_only
rule aborts the HTTP request, so no request is ever sent.
"""
import argparse
import datetime
import socket
import ssl
import sys

ORIGIN = '158.180.58.90'
HOSTS = ('oteryn.com', 'www.oteryn.com', 'api.oteryn.com', 'login.oteryn.com',
         'gateway.oteryn.com', 'game.oteryn.com')
MIN_DAYS = 14


def tls_context(cafile=None):
    """Verifying client context that accepts only TLS 1.3, which every edge host serves."""
    context = ssl.create_default_context(cafile=cafile)
    context.minimum_version = ssl.TLSVersion.TLSv1_3
    return context


def days_left(not_after, now):
    expires = datetime.datetime.fromtimestamp(ssl.cert_time_to_seconds(not_after), datetime.timezone.utc)
    return (expires - now).total_seconds() / 86400


def check(host, origin, context, now, port=443, timeout=10.0):
    """One host: (ok, line). Verification failures count as not ok."""
    try:
        with socket.create_connection((origin, port), timeout=timeout) as raw:
            with context.wrap_socket(raw, server_hostname=host) as tls:
                cert = tls.getpeercert()
    except (OSError, ssl.SSLError) as error:
        return False, f'{host}: FAIL {type(error).__name__}: {error}'
    left = days_left(cert['notAfter'], now)
    issuer = dict(item[0] for item in cert.get('issuer', ())).get('organizationName', '?')
    ok = left >= MIN_DAYS
    return ok, f'{host}: {"ok" if ok else "FAIL"} {left:.1f} days left (notAfter {cert["notAfter"]}, {issuer})'


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--origin', default=ORIGIN)
    parser.add_argument('hosts', nargs='*', default=HOSTS)
    args = parser.parse_args(argv)
    context = tls_context()
    now = datetime.datetime.now(datetime.timezone.utc)
    failed = 0
    for host in args.hosts:
        ok, line = check(host, args.origin, context, now)
        print(line)
        failed += not ok
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
