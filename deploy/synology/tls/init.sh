#!/bin/sh
set -eu

TLS_DIR="${TLS_DIR:-/tls}"
# PostgreSQL (ADR 0046 §6, plan P4): the server certificate for the Compose service name `postgres`, signed
# by the same stack CA, goes to its own volume with only the server pair and the CA certificate; the CA
# certificate alone (never a key) goes to the client volume mounted read-only by Platform. Both are
# optional so the script also runs where Compose does not mount them.
POSTGRES_TLS_DIR="${POSTGRES_TLS_DIR:-}"
POSTGRES_CA_DIR="${POSTGRES_CA_DIR:-}"
# The postgres user of the pinned Debian postgres image; the server refuses a key it cannot own.
POSTGRES_UID="${POSTGRES_UID:-999}"
BOOTSTRAP_DIR="${BOOTSTRAP_DIR:-}"
mkdir -p "$TLS_DIR"

certificates_are_current() {
    [ -s "$TLS_DIR/ca.crt" ] \
        && [ -s "$TLS_DIR/ca.key" ] \
        && [ -s "$TLS_DIR/platform-internal.crt" ] \
        && [ -s "$TLS_DIR/platform-internal.key" ] \
        && openssl x509 -checkend 604800 -noout -in "$TLS_DIR/ca.crt" >/dev/null 2>&1 \
        && openssl x509 -checkend 604800 -noout -in "$TLS_DIR/platform-internal.crt" >/dev/null 2>&1
}

# Current means: present, valid for at least 7 more days, issued by the current CA, and both CA copies
# equal to the current CA certificate.
postgres_certificate_is_current() {
    [ -n "$POSTGRES_TLS_DIR" ] || return 0
    [ -s "$POSTGRES_TLS_DIR/server.crt" ] \
        && [ -s "$POSTGRES_TLS_DIR/server.key" ] \
        && cmp -s "$TLS_DIR/ca.crt" "$POSTGRES_TLS_DIR/ca.crt" \
        && { [ -z "$POSTGRES_CA_DIR" ] || cmp -s "$TLS_DIR/ca.crt" "$POSTGRES_CA_DIR/ca.crt"; } \
        && openssl x509 -checkend 604800 -noout -in "$POSTGRES_TLS_DIR/server.crt" >/dev/null 2>&1 \
        && openssl verify -CAfile "$TLS_DIR/ca.crt" "$POSTGRES_TLS_DIR/server.crt" >/dev/null 2>&1
}

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

# Issues the postgres server pair from the current CA and moves it out of TLS_DIR, so no service that
# mounts internal_tls ever sees it. The key is written owner-only for the postgres user.
issue_postgres_certificate() {
    [ -n "$POSTGRES_TLS_DIR" ] || return 0
    mkdir -p "$POSTGRES_TLS_DIR"
    # umask 077 would leave a directory created here unreadable to the postgres user; the key itself
    # stays owner-only.
    chmod 0755 "$POSTGRES_TLS_DIR"
    issue_certificate postgres
    rm -f "$POSTGRES_TLS_DIR/server.key" "$POSTGRES_TLS_DIR/server.crt" "$POSTGRES_TLS_DIR/ca.crt"
    mv "$TLS_DIR/postgres.key" "$POSTGRES_TLS_DIR/server.key"
    mv "$TLS_DIR/postgres.crt" "$POSTGRES_TLS_DIR/server.crt"
    cp "$TLS_DIR/ca.crt" "$POSTGRES_TLS_DIR/ca.crt"
    chown "$POSTGRES_UID:$POSTGRES_UID" "$POSTGRES_TLS_DIR/server.key" "$POSTGRES_TLS_DIR/server.crt"
    chmod 0600 "$POSTGRES_TLS_DIR/server.key"
    chmod 0444 "$POSTGRES_TLS_DIR/server.crt" "$POSTGRES_TLS_DIR/ca.crt"
    if [ -n "$POSTGRES_CA_DIR" ]; then
        mkdir -p "$POSTGRES_CA_DIR"
        chmod 0755 "$POSTGRES_CA_DIR"
        rm -f "$POSTGRES_CA_DIR/ca.crt"
        cp "$TLS_DIR/ca.crt" "$POSTGRES_CA_DIR/ca.crt"
        chmod 0444 "$POSTGRES_CA_DIR/ca.crt"
    fi
    rm -f "$TLS_DIR"/*.csr "$TLS_DIR"/*.ext "$TLS_DIR"/*.srl
}

# deploy.sh stages the PostgreSQL configuration and role script with docker cp; make them readable by the
# postgres user, which mounts bootstrap_files read-only. They hold no secret.
publish_postgres_bootstrap_files() {
    [ -n "$BOOTSTRAP_DIR" ] || return 0
    for file in postgres-postgresql.conf postgres-pg_hba.conf postgres-roles.sh; do
        if [ -f "$BOOTSTRAP_DIR/$file" ]; then
            chmod 0644 "$BOOTSTRAP_DIR/$file"
        fi
    done
}

umask 077
publish_postgres_bootstrap_files

if certificates_are_current; then
    postgres_certificate_is_current || issue_postgres_certificate
    exit 0
fi

rm -f "$TLS_DIR"/*.key "$TLS_DIR"/*.crt "$TLS_DIR"/*.csr "$TLS_DIR"/*.ext "$TLS_DIR"/*.srl

openssl genrsa -out "$TLS_DIR/ca.key" 4096
openssl req -x509 -new -sha256 -days 30 \
    -key "$TLS_DIR/ca.key" \
    -subj "/CN=Oteryn Synology Staging Internal CA" \
    -out "$TLS_DIR/ca.crt"

issue_certificate platform-internal

rm -f "$TLS_DIR"/*.csr "$TLS_DIR"/*.ext "$TLS_DIR"/*.srl
chmod 0444 "$TLS_DIR"/*.crt
chmod 0400 "$TLS_DIR"/*.key

issue_postgres_certificate
