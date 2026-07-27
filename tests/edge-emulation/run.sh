#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work_dir="${EDGE_EMULATION_WORK_DIR:-/tmp/oteryn-edge-emulation}"
evidence_path="${EDGE_EVIDENCE_PATH:-$repo_root/artifacts/edge-security-emulation.json}"
validation_sha="${VALIDATION_SHA:-$(git -C "$repo_root" rev-parse HEAD)}"
dns_container="oteryn-edge-dns-${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}"
access_pid=""
nginx_started=0

fail() {
    echo "edge-emulation failure: $*" >&2
    exit 1
}

cleanup() {
    status=$?
    set +e
    if (( nginx_started == 1 )); then
        nginx -p "$work_dir/" -c "$work_dir/nginx.conf" -s quit >/dev/null 2>&1
    fi
    if [[ -n "$access_pid" ]]; then
        kill "$access_pid" >/dev/null 2>&1
        wait "$access_pid" >/dev/null 2>&1
    fi
    docker rm -f "$dns_container" >/dev/null 2>&1
    if (( status != 0 )); then
        echo "--- nginx error log ---" >&2
        tail -n 120 "$work_dir/nginx-error.log" 2>/dev/null >&2
        echo "--- access verifier log ---" >&2
        tail -n 80 "$work_dir/access.log" 2>/dev/null >&2
        echo "--- laravel log ---" >&2
        tail -n 120 "${LARAVEL_EDGE_LOG:-/tmp/oteryn-edge-laravel.log}" 2>/dev/null >&2
    fi
    exit "$status"
}
trap cleanup EXIT

rm -rf "$work_dir"
mkdir -p "$work_dir/certs" "$(dirname "$evidence_path")"
cert_dir="$work_dir/certs"

for command in curl dig docker nginx openssl python3 ss; do
    command -v "$command" >/dev/null || fail "required command is missing: $command"
done

curl --fail --silent --show-error http://127.0.0.1:8000/health >/dev/null \
    || fail "current-SHA Laravel health endpoint is unavailable"

openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 2 \
    -subj "/CN=Oteryn Edge Emulation CA" \
    -keyout "$cert_dir/edge-ca.key" -out "$cert_dir/edge-ca.crt" >/dev/null 2>&1
openssl req -newkey rsa:2048 -sha256 -nodes \
    -subj "/CN=app.oteryn.test" \
    -keyout "$cert_dir/edge.key" -out "$cert_dir/edge.csr" >/dev/null 2>&1
cat > "$cert_dir/edge.ext" <<'EOF'
subjectAltName=DNS:app.oteryn.test,DNS:admin.oteryn.test
basicConstraints=CA:FALSE
keyUsage=digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
EOF
openssl x509 -req -sha256 -days 2 \
    -in "$cert_dir/edge.csr" -CA "$cert_dir/edge-ca.crt" -CAkey "$cert_dir/edge-ca.key" \
    -CAcreateserial -extfile "$cert_dir/edge.ext" -out "$cert_dir/edge.crt" >/dev/null 2>&1

openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 2 \
    -subj "/CN=Oteryn Origin Emulation CA" \
    -keyout "$cert_dir/origin-ca.key" -out "$cert_dir/origin-ca.crt" >/dev/null 2>&1
openssl req -newkey rsa:2048 -sha256 -nodes \
    -subj "/CN=origin.oteryn.internal" \
    -keyout "$cert_dir/origin.key" -out "$cert_dir/origin.csr" >/dev/null 2>&1
cat > "$cert_dir/origin.ext" <<'EOF'
subjectAltName=DNS:origin.oteryn.internal,IP:127.0.0.1
basicConstraints=CA:FALSE
keyUsage=digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
EOF
openssl x509 -req -sha256 -days 2 \
    -in "$cert_dir/origin.csr" -CA "$cert_dir/origin-ca.crt" -CAkey "$cert_dir/origin-ca.key" \
    -CAcreateserial -extfile "$cert_dir/origin.ext" -out "$cert_dir/origin.crt" >/dev/null 2>&1
openssl req -newkey rsa:2048 -sha256 -nodes \
    -subj "/CN=oteryn-edge-client" \
    -keyout "$cert_dir/edge-client.key" -out "$cert_dir/edge-client.csr" >/dev/null 2>&1
cat > "$cert_dir/edge-client.ext" <<'EOF'
basicConstraints=CA:FALSE
keyUsage=digitalSignature,keyEncipherment
extendedKeyUsage=clientAuth
EOF
openssl x509 -req -sha256 -days 2 \
    -in "$cert_dir/edge-client.csr" -CA "$cert_dir/origin-ca.crt" -CAkey "$cert_dir/origin-ca.key" \
    -CAcreateserial -extfile "$cert_dir/edge-client.ext" -out "$cert_dir/edge-client.crt" >/dev/null 2>&1
chmod 600 "$cert_dir"/*.key

access_secret="$(openssl rand -hex 32)"
ACCESS_JWT_SECRET="$access_secret" python3 "$repo_root/tests/edge-emulation/access_verifier.py" \
    >"$work_dir/access.log" 2>&1 &
access_pid=$!

cat > "$work_dir/nginx.conf" <<EOF
pid $work_dir/nginx.pid;
error_log $work_dir/nginx-error.log info;

events {}

http {
    access_log $work_dir/nginx-access.log;
    server_tokens off;

    map \$request_uri \$waf_block {
        default 0;
        ~*(\.\./|%2e%2e|%252e%252e|<script|%3cscript|union(%20|\+|[[:space:]])+select|/etc/passwd) 1;
    }

    map \$request_method \$method_block {
        default 1;
        GET 0;
        HEAD 0;
        POST 0;
    }

    limit_req_zone \$binary_remote_addr zone=edge_rate:10m rate=2r/s;

    server {
        listen 127.0.0.1:9443 ssl;
        server_name origin.oteryn.internal;
        ssl_certificate $cert_dir/origin.crt;
        ssl_certificate_key $cert_dir/origin.key;
        ssl_client_certificate $cert_dir/origin-ca.crt;
        ssl_verify_client on;
        ssl_verify_depth 2;
        ssl_protocols TLSv1.2 TLSv1.3;

        location = /__edge_probe {
            default_type application/json;
            return 200 '{"cf_connecting_ip":"\$http_cf_connecting_ip","forwarded_proto":"\$http_x_forwarded_proto","host":"\$host"}';
        }

        location / {
            proxy_pass http://127.0.0.1:8000;
            proxy_http_version 1.1;
            proxy_set_header Host app.oteryn.test;
            proxy_set_header X-Forwarded-Proto https;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_set_header CF-Connecting-IP \$http_cf_connecting_ip;
            proxy_set_header Connection "";
        }
    }

    server {
        listen 127.0.0.1:8080;
        server_name app.oteryn.test admin.oteryn.test;
        return 308 https://\$host:8443\$request_uri;
    }

    server {
        listen 127.0.0.1:8443 ssl;
        server_name app.oteryn.test admin.oteryn.test;
        ssl_certificate $cert_dir/edge.crt;
        ssl_certificate_key $cert_dir/edge.key;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_session_cache off;
        client_max_body_size 64k;
        limit_req_status 429;

        add_header CF-Ray "edge-emulation-\$request_id" always;
        add_header CF-Cache-Status "DYNAMIC" always;
        add_header Strict-Transport-Security "max-age=86400" always;
        add_header X-Edge-Emulation "cloudflare-waf-access" always;

        if (\$waf_block) { return 403; }
        if (\$method_block) { return 405; }

        location = /_access_check {
            internal;
            proxy_pass http://127.0.0.1:9080;
            proxy_pass_request_body off;
            proxy_set_header Content-Length "";
            proxy_set_header X-Access-JWT \$http_cf_access_jwt_assertion;
        }

        location = /__rate_limit_probe {
            limit_req zone=edge_rate burst=1 nodelay;
            proxy_pass https://127.0.0.1:9443/__edge_probe;
            proxy_ssl_server_name on;
            proxy_ssl_name origin.oteryn.internal;
            proxy_ssl_trusted_certificate $cert_dir/origin-ca.crt;
            proxy_ssl_verify on;
            proxy_ssl_verify_depth 2;
            proxy_ssl_certificate $cert_dir/edge-client.crt;
            proxy_ssl_certificate_key $cert_dir/edge-client.key;
            proxy_set_header Host origin.oteryn.internal;
            proxy_set_header X-Forwarded-Proto https;
            proxy_set_header CF-Connecting-IP \$remote_addr;
        }

        location ^~ /admin {
            auth_request /_access_check;
            proxy_pass https://127.0.0.1:9443;
            proxy_ssl_server_name on;
            proxy_ssl_name origin.oteryn.internal;
            proxy_ssl_trusted_certificate $cert_dir/origin-ca.crt;
            proxy_ssl_verify on;
            proxy_ssl_verify_depth 2;
            proxy_ssl_certificate $cert_dir/edge-client.crt;
            proxy_ssl_certificate_key $cert_dir/edge-client.key;
            proxy_set_header Host app.oteryn.test;
            proxy_set_header X-Forwarded-Proto https;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_set_header CF-Connecting-IP \$remote_addr;
        }

        location / {
            proxy_pass https://127.0.0.1:9443;
            proxy_ssl_server_name on;
            proxy_ssl_name origin.oteryn.internal;
            proxy_ssl_trusted_certificate $cert_dir/origin-ca.crt;
            proxy_ssl_verify on;
            proxy_ssl_verify_depth 2;
            proxy_ssl_certificate $cert_dir/edge-client.crt;
            proxy_ssl_certificate_key $cert_dir/edge-client.key;
            proxy_set_header Host app.oteryn.test;
            proxy_set_header X-Forwarded-Proto https;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_set_header CF-Connecting-IP \$remote_addr;
        }
    }
}
EOF

nginx -t -p "$work_dir/" -c "$work_dir/nginx.conf"
nginx -p "$work_dir/" -c "$work_dir/nginx.conf"
nginx_started=1

docker run --detach --rm --name "$dns_container" \
    --publish 127.0.0.1:1053:53/udp \
    --publish 127.0.0.1:1053:53/tcp \
    --volume "$repo_root/tests/edge-emulation/Corefile:/Corefile:ro" \
    --volume "$repo_root/tests/edge-emulation/db.oteryn.test:/zones/db.oteryn.test:ro" \
    coredns/coredns:1.12.1 -conf /Corefile >/dev/null

for _ in $(seq 1 40); do
    if curl --silent http://127.0.0.1:9080 >/dev/null 2>&1 \
        && curl --silent --cacert "$cert_dir/edge-ca.crt" \
            --resolve app.oteryn.test:8443:127.0.0.1 \
            https://app.oteryn.test:8443/health >/dev/null 2>&1 \
        && dig +time=1 +tries=1 @127.0.0.1 -p 1053 app.oteryn.test A >/dev/null 2>&1; then
        break
    fi
    sleep 0.25
done

[[ "$(dig +short @127.0.0.1 -p 1053 app.oteryn.test A | tr -d '\r')" == "127.0.0.1" ]] \
    || fail "app A record does not resolve to the edge fixture"
[[ "$(dig +short @127.0.0.1 -p 1053 admin.oteryn.test CNAME | tr -d '\r')" == "app.oteryn.test." ]] \
    || fail "admin CNAME does not resolve to the public edge name"
dig +tcp @127.0.0.1 -p 1053 app.oteryn.test A | grep -q 'status: NOERROR' \
    || fail "DNS-over-TCP query failed"
dig @127.0.0.1 -p 1053 absent.oteryn.test A | grep -q 'status: NXDOMAIN' \
    || fail "unconfigured name did not fail closed with NXDOMAIN"

ss -ltn | grep -q '127.0.0.1:9443' || fail "origin is not loopback-bound"
if ss -ltn | grep -Eq '(0\.0\.0\.0|\[::\]):9443'; then
    fail "origin unexpectedly listens on a wildcard address"
fi
openssl verify -CAfile "$cert_dir/edge-ca.crt" "$cert_dir/edge.crt" >/dev/null
openssl verify -CAfile "$cert_dir/origin-ca.crt" "$cert_dir/origin.crt" >/dev/null
openssl s_client -connect 127.0.0.1:8443 -servername app.oteryn.test \
    -CAfile "$cert_dir/edge-ca.crt" -verify_return_error -tls1_2 </dev/null 2>/dev/null \
    | grep -q 'Verify return code: 0 (ok)' || fail "TLS 1.2 edge verification failed"
openssl s_client -connect 127.0.0.1:8443 -servername app.oteryn.test \
    -CAfile "$cert_dir/edge-ca.crt" -verify_return_error -tls1_3 </dev/null 2>/dev/null \
    | grep -q 'Verify return code: 0 (ok)' || fail "TLS 1.3 edge verification failed"
if openssl s_client -connect 127.0.0.1:8443 -servername app.oteryn.test -tls1 </dev/null >/dev/null 2>&1; then
    fail "legacy TLS 1.0 unexpectedly succeeded"
fi

redirect_headers="$work_dir/redirect.headers"
curl --silent --show-error --dump-header "$redirect_headers" --output /dev/null \
    --resolve app.oteryn.test:8080:127.0.0.1 http://app.oteryn.test:8080/health
head -n 1 "$redirect_headers" | grep -Eq ' 308 ' || fail "HTTP edge did not return 308"
grep -Eiq '^location: https://app\.oteryn\.test:8443/health\r?$' "$redirect_headers" \
    || fail "HTTP redirect target is incorrect"

edge_headers="$work_dir/edge.headers"
edge_status="$(curl --silent --show-error --cacert "$cert_dir/edge-ca.crt" \
    --resolve app.oteryn.test:8443:127.0.0.1 \
    --dump-header "$edge_headers" --output "$work_dir/health.body" --write-out '%{http_code}' \
    https://app.oteryn.test:8443/health)"
[[ "$edge_status" == "200" ]] || fail "edge health returned HTTP $edge_status"
grep -Eiq '^cf-ray: edge-emulation-' "$edge_headers" || fail "CF-Ray emulation header is absent"
grep -Eiq '^cf-cache-status: DYNAMIC\r?$' "$edge_headers" || fail "CF-Cache-Status header is absent"
grep -Eiq '^strict-transport-security: max-age=86400\r?$' "$edge_headers" || fail "bounded HSTS emulation header is absent"

probe_body="$(curl --silent --show-error --cacert "$cert_dir/edge-ca.crt" \
    --resolve app.oteryn.test:8443:127.0.0.1 \
    -H 'CF-Connecting-IP: 203.0.113.99' \
    https://app.oteryn.test:8443/__edge_probe)"
python3 - "$probe_body" <<'PY'
import json
import sys

payload = json.loads(sys.argv[1])
assert payload["cf_connecting_ip"] == "127.0.0.1", payload
assert payload["forwarded_proto"] == "https", payload
assert payload["host"] == "app.oteryn.test", payload
PY

set +e
origin_without_cert="$(curl --silent --show-error --cacert "$cert_dir/origin-ca.crt" \
    --resolve origin.oteryn.internal:9443:127.0.0.1 \
    --output /dev/null --write-out '%{http_code}' \
    https://origin.oteryn.internal:9443/health 2>/dev/null)"
origin_without_cert_rc=$?
set -e
if (( origin_without_cert_rc == 0 )) && [[ "$origin_without_cert" == "200" ]]; then
    fail "direct origin unexpectedly succeeded without an authenticated edge certificate"
fi
origin_with_cert="$(curl --silent --show-error --cacert "$cert_dir/origin-ca.crt" \
    --cert "$cert_dir/edge-client.crt" --key "$cert_dir/edge-client.key" \
    --resolve origin.oteryn.internal:9443:127.0.0.1 \
    --output /dev/null --write-out '%{http_code}' \
    https://origin.oteryn.internal:9443/health)"
[[ "$origin_with_cert" == "200" ]] || fail "authenticated origin pull returned HTTP $origin_with_cert"

for probe in \
    '/../../etc/passwd' \
    '/?q=%3Cscript%3Ealert(1)%3C/script%3E' \
    '/?q=union%20select%20password'; do
    status="$(curl --path-as-is --silent --show-error --cacert "$cert_dir/edge-ca.crt" \
        --resolve app.oteryn.test:8443:127.0.0.1 \
        --output /dev/null --write-out '%{http_code}' \
        "https://app.oteryn.test:8443$probe")"
    [[ "$status" == "400" || "$status" == "403" ]] \
        || fail "WAF probe $probe returned HTTP $status"
done
method_status="$(curl --silent --show-error --cacert "$cert_dir/edge-ca.crt" \
    --resolve app.oteryn.test:8443:127.0.0.1 \
    --request TRACE --output /dev/null --write-out '%{http_code}' \
    https://app.oteryn.test:8443/health)"
[[ "$method_status" == "405" ]] || fail "unsupported method returned HTTP $method_status"
head -c 70000 /dev/zero > "$work_dir/oversized.bin"
body_status="$(curl --silent --show-error --cacert "$cert_dir/edge-ca.crt" \
    --resolve app.oteryn.test:8443:127.0.0.1 \
    --request POST --data-binary @"$work_dir/oversized.bin" \
    --output /dev/null --write-out '%{http_code}' \
    https://app.oteryn.test:8443/login)"
[[ "$body_status" == "413" ]] || fail "oversized request returned HTTP $body_status"

: > "$work_dir/rate-statuses"
for _ in $(seq 1 24); do
    (
        curl --silent --show-error --cacert "$cert_dir/edge-ca.crt" \
            --resolve app.oteryn.test:8443:127.0.0.1 \
            --output /dev/null --write-out '%{http_code}\n' \
            https://app.oteryn.test:8443/__rate_limit_probe \
            >> "$work_dir/rate-statuses"
    ) &
done
wait
grep -Eq '^(200|204)$' "$work_dir/rate-statuses" || fail "rate-limit probe admitted no request"
grep -qx '429' "$work_dir/rate-statuses" || fail "rate-limit probe produced no HTTP 429"

missing_access_status="$(curl --silent --show-error --cacert "$cert_dir/edge-ca.crt" \
    --resolve admin.oteryn.test:8443:127.0.0.1 \
    --output /dev/null --write-out '%{http_code}' \
    https://admin.oteryn.test:8443/admin)"
[[ "$missing_access_status" == "401" || "$missing_access_status" == "403" ]] \
    || fail "missing Access assertion returned HTTP $missing_access_status"
invalid_access_status="$(curl --silent --show-error --cacert "$cert_dir/edge-ca.crt" \
    --resolve admin.oteryn.test:8443:127.0.0.1 \
    -H 'CF-Access-Jwt-Assertion: invalid' \
    --output /dev/null --write-out '%{http_code}' \
    https://admin.oteryn.test:8443/admin)"
[[ "$invalid_access_status" == "401" || "$invalid_access_status" == "403" ]] \
    || fail "invalid Access assertion returned HTTP $invalid_access_status"
valid_access_token="$(ACCESS_JWT_SECRET="$access_secret" python3 - <<'PY'
import base64
import hashlib
import hmac
import json
import os
import time


def encode(value):
    raw = json.dumps(value, separators=(",", ":")).encode()
    return base64.urlsafe_b64encode(raw).rstrip(b"=").decode()

header = encode({"alg": "HS256", "typ": "JWT"})
payload = encode({"aud": "oteryn-admin", "email": "operator@example.test", "exp": int(time.time()) + 300})
signed = f"{header}.{payload}"
signature = base64.urlsafe_b64encode(
    hmac.new(os.environ["ACCESS_JWT_SECRET"].encode(), signed.encode(), hashlib.sha256).digest()
).rstrip(b"=").decode()
print(f"{signed}.{signature}")
PY
)"
valid_headers="$work_dir/admin-valid.headers"
valid_access_status="$(curl --silent --show-error --cacert "$cert_dir/edge-ca.crt" \
    --resolve admin.oteryn.test:8443:127.0.0.1 \
    -H "CF-Access-Jwt-Assertion: $valid_access_token" \
    --dump-header "$valid_headers" --output /dev/null --write-out '%{http_code}' \
    https://admin.oteryn.test:8443/admin)"
[[ "$valid_access_status" == "302" || "$valid_access_status" == "303" ]] \
    || fail "valid Access assertion did not reach Platform auth; HTTP $valid_access_status"
grep -Eiq '^location: .*login' "$valid_headers" \
    || fail "Platform authentication did not remain authoritative after Access admission"

cat > "$evidence_path" <<EOF
{
  "classification": "STAGING_PROVEN",
  "validation_sha": "$validation_sha",
  "profile": "edge-security-emulation",
  "dns_authoritative_fixture": "PASS",
  "dns_negative_answer": "PASS",
  "edge_tls_1_2": "PASS",
  "edge_tls_1_3": "PASS",
  "legacy_tls_denial": "PASS",
  "http_to_https_redirect": "PASS",
  "current_sha_laravel_health_through_edge": "PASS",
  "authenticated_origin_pull": "PASS",
  "direct_origin_without_client_certificate": "DENIED",
  "spoofed_client_ip_header": "OVERWRITTEN",
  "public_host_preserved_to_origin": "PASS",
  "cloudflare_style_metadata": "PASS",
  "waf_traversal_xss_sqli": "DENIED",
  "unsupported_method": "DENIED",
  "oversized_body": "DENIED",
  "bounded_rate_limit": "HTTP_429_OBSERVED",
  "access_missing_or_invalid_assertion": "DENIED",
  "platform_auth_after_access_admission": "REQUIRED",
  "production_environment_proven": false
}
EOF

echo "EDGE_SECURITY_EMULATION_STAGING_PROVEN sha=$validation_sha evidence=$evidence_path"
