#!/bin/sh
set -eu

TLS_DIR="${TLS_DIR:-/tls}"
mkdir -p "$TLS_DIR"

certificates_are_current() {
    [ -s "$TLS_DIR/ca.crt" ] \
        && [ -s "$TLS_DIR/ca.key" ] \
        && [ -s "$TLS_DIR/platform-internal.crt" ] \
        && [ -s "$TLS_DIR/platform-internal.key" ] \
        && openssl x509 -checkend 604800 -noout -in "$TLS_DIR/ca.crt" >/dev/null 2>&1 \
        && openssl x509 -checkend 604800 -noout -in "$TLS_DIR/platform-internal.crt" >/dev/null 2>&1
}

if certificates_are_current; then
    exit 0
fi

umask 077
rm -f "$TLS_DIR"/*.key "$TLS_DIR"/*.crt "$TLS_DIR"/*.csr "$TLS_DIR"/*.ext "$TLS_DIR"/*.srl

openssl genrsa -out "$TLS_DIR/ca.key" 4096
openssl req -x509 -new -sha256 -days 30 \
    -key "$TLS_DIR/ca.key" \
    -subj "/CN=Oteryn Synology Staging Internal CA" \
    -out "$TLS_DIR/ca.crt"

issue_certificate() {
    name="$1"

    openssl genrsa -out "$TLS_DIR/$name.key" 2048
    openssl req -new \
        -key "$TLS_DIR/$name.key" \
        -subj "/CN=$name" \
        -out "$TLS_DIR/$name.csr"

    cat > "$TLS_DIR/$name.ext" <<EOF
subjectAltName=DNS:$name
extendedKeyUsage=serverAuth
keyUsage=digitalSignature,keyEncipherment
EOF

    openssl x509 -req -sha256 -days 30 \
        -in "$TLS_DIR/$name.csr" \
        -CA "$TLS_DIR/ca.crt" \
        -CAkey "$TLS_DIR/ca.key" \
        -CAcreateserial \
        -extfile "$TLS_DIR/$name.ext" \
        -out "$TLS_DIR/$name.crt"
}

issue_certificate platform-internal

rm -f "$TLS_DIR"/*.csr "$TLS_DIR"/*.ext "$TLS_DIR"/*.srl
chmod 0444 "$TLS_DIR"/*.crt
chmod 0400 "$TLS_DIR"/*.key
