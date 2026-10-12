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

# A certificate and a private key belong together: equal public keys.
pair_matches() {
    certificate_key="$(openssl x509 -noout -pubkey -in "$1" 2>/dev/null)" || return 1
    private_key="$(openssl pkey -pubout -in "$2" 2>/dev/null)" || return 1
    [ -n "$certificate_key" ] && [ "$certificate_key" = "$private_key" ]
}

# Current means: present, valid for at least 7 more days, issued by the CA, and a matching pair (presence
# and expiry alone pass a key of another certificate or a certificate of another CA). The CA key must
# belong to the CA certificate.
certificates_are_current() {
    [ -s "$TLS_DIR/ca.crt" ] \
        && [ -s "$TLS_DIR/ca.key" ] \
        && [ -s "$TLS_DIR/platform-internal.crt" ] \
        && [ -s "$TLS_DIR/platform-internal.key" ] \
        && openssl x509 -checkend 604800 -noout -in "$TLS_DIR/ca.crt" >/dev/null 2>&1 \
        && openssl x509 -checkend 604800 -noout -in "$TLS_DIR/platform-internal.crt" >/dev/null 2>&1 \
        && openssl verify -CAfile "$TLS_DIR/ca.crt" "$TLS_DIR/platform-internal.crt" >/dev/null 2>&1 \
        && pair_matches "$TLS_DIR/ca.crt" "$TLS_DIR/ca.key" \
        && pair_matches "$TLS_DIR/platform-internal.crt" "$TLS_DIR/platform-internal.key"
}

# Current means: present, valid for at least 7 more days, issued by the current CA, the certificate and the
# private key a matching pair (an interrupted or manual rotation can leave a nonempty key of another
# certificate, which a reload would not notice but the next container start would refuse), and both CA
# copies equal to the current CA certificate.
postgres_pair_matches() {
    pair_matches "$POSTGRES_TLS_DIR/server.crt" "$POSTGRES_TLS_DIR/server.key"
}

# The key must stay owner-only and owned by the postgres user, or the server refuses to start.
postgres_key_is_protected() {
    [ "$(stat -c '%u %a' "$POSTGRES_TLS_DIR/server.key" 2>/dev/null)" = "$POSTGRES_UID 600" ]
}

postgres_certificate_is_current() {
    [ -n "$POSTGRES_TLS_DIR" ] || return 0
    [ -s "$POSTGRES_TLS_DIR/server.crt" ] \
        && [ -s "$POSTGRES_TLS_DIR/server.key" ] \
        && cmp -s "$TLS_DIR/ca.crt" "$POSTGRES_TLS_DIR/ca.crt" \
        && { [ -z "$POSTGRES_CA_DIR" ] || cmp -s "$TLS_DIR/ca.crt" "$POSTGRES_CA_DIR/ca.crt"; } \
        && openssl x509 -checkend 604800 -noout -in "$POSTGRES_TLS_DIR/server.crt" >/dev/null 2>&1 \
        && openssl verify -CAfile "$TLS_DIR/ca.crt" "$POSTGRES_TLS_DIR/server.crt" >/dev/null 2>&1 \
        && postgres_pair_matches \
        && postgres_key_is_protected
}

# The deploy fast path (scripts/lib.sh) runs this same predicate inside the running postgres container, where
# only the volume it serves is visible: `sh -s -- --check-postgres-pair < init.sh` with POSTGRES_TLS_DIR set
# to that volume. The stack CA is then the CA copy next to the server pair; a CA rotated since is caught
# because tls-init, which a full deploy always runs, reissues the pair and both copies in the same run.
if [ "${1:-}" = --check-postgres-pair ]; then
    TLS_DIR="$POSTGRES_TLS_DIR"
    POSTGRES_CA_DIR=''
    postgres_certificate_is_current
    exit $?
fi

mkdir -p "$TLS_DIR"

issue_certificate() {
    name="$1"
    work="${2:-$TLS_DIR}"

    openssl genrsa -out "$work/$name.key" 2048
    openssl req -new \
        -key "$work/$name.key" \
        -subj "/CN=$name" \
        -out "$work/$name.csr"

    cat > "$work/$name.ext" <<EOF
subjectAltName=DNS:$name
extendedKeyUsage=serverAuth
keyUsage=digitalSignature,keyEncipherment
EOF

    openssl x509 -req -sha256 -days 30 \
        -in "$work/$name.csr" \
        -CA "$TLS_DIR/ca.crt" \
        -CAkey "$TLS_DIR/ca.key" \
        -CAcreateserial \
        -extfile "$work/$name.ext" \
        -out "$work/$name.crt"
}

# Issues the postgres server pair from the current CA and moves it out of TLS_DIR, so no service that
# mounts internal_tls ever sees it. The key is written owner-only for the postgres user.
issue_postgres_certificate() {
    [ -n "$POSTGRES_TLS_DIR" ] || return 0
    mkdir -p "$POSTGRES_TLS_DIR"
    # umask 077 would leave a directory created here unreadable to the postgres user; the key itself
    # stays owner-only.
    chmod 0755 "$POSTGRES_TLS_DIR"
    # The key is generated and the certificate signed inside the PostgreSQL volume, never in TLS_DIR: the
    # ingress containers mount that read-only and must not see even a transient postgres.key. Each file is
    # finished (owner, mode) in the work directory and then renamed into place, so a reader never sees a
    # half-written file; an interrupted run is caught by postgres_certificate_is_current (missing, corrupt
    # or mismatched pair) and reissued.
    rm -rf "$POSTGRES_TLS_DIR"/.issue.*
    work="$(mktemp -d "$POSTGRES_TLS_DIR/.issue.XXXXXX")"
    issue_certificate postgres "$work"
    cp "$TLS_DIR/ca.crt" "$work/ca.crt"
    chown "$POSTGRES_UID:$POSTGRES_UID" "$work/postgres.key" "$work/postgres.crt"
    chmod 0600 "$work/postgres.key"
    chmod 0444 "$work/postgres.crt" "$work/ca.crt"
    mv "$work/postgres.key" "$POSTGRES_TLS_DIR/server.key"
    mv "$work/postgres.crt" "$POSTGRES_TLS_DIR/server.crt"
    mv "$work/ca.crt" "$POSTGRES_TLS_DIR/ca.crt"
    rm -rf "$work"
    if [ -n "$POSTGRES_CA_DIR" ]; then
        mkdir -p "$POSTGRES_CA_DIR"
        chmod 0755 "$POSTGRES_CA_DIR"
        rm -f "$POSTGRES_CA_DIR/.ca.crt.new"
        cp "$TLS_DIR/ca.crt" "$POSTGRES_CA_DIR/.ca.crt.new"
        chmod 0444 "$POSTGRES_CA_DIR/.ca.crt.new"
        mv "$POSTGRES_CA_DIR/.ca.crt.new" "$POSTGRES_CA_DIR/ca.crt"
    fi
    rm -f "$TLS_DIR"/*.srl
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
