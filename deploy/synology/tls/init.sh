#!/bin/sh
set -eu

TLS_DIR="${TLS_DIR:-/tls}"
# PostgreSQL (ADR 0046 §6, plan P4): the server certificate for the Compose service name `postgres`, signed
# by the same stack CA, goes to its own volume with only the server pair and the CA certificate; the CA
# certificate alone (never a key) goes to the client volume mounted read-only by Platform. Both are
# optional so the script also runs where Compose does not mount them.
POSTGRES_TLS_DIR="${POSTGRES_TLS_DIR:-}"
POSTGRES_CA_DIR="${POSTGRES_CA_DIR:-}"
# The PostgreSQL server certificate is signed by its own CA, not by the stack CA: the stack CA key lives in
# TLS_DIR, which the root-started ingress containers mount, and anything that can read it could mint a
# certificate Platform accepts for `postgres`. The PostgreSQL CA key lives only in POSTGRES_SIGNER_DIR, a
# volume mounted by this script alone; the other volumes carry its public certificate only.
POSTGRES_SIGNER_DIR="${POSTGRES_SIGNER_DIR:-}"
PG_CA_SUBJECT="Oteryn Synology Staging PostgreSQL CA"
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

# Current means: present, valid for at least 7 more days, issued by the current PostgreSQL CA, the certificate and the
# private key a matching pair (an interrupted or manual rotation can leave a nonempty key of another
# certificate, which a reload would not notice but the next container start would refuse), and both CA
# copies equal to the current CA certificate.
postgres_ca_cert() {
    printf '%s/pgca.crt' "$POSTGRES_SIGNER_DIR"
}

# The PostgreSQL CA is current when its certificate and key are present, a matching pair, valid for 7 more
# days, and the dedicated CA (by subject). Anything else, including a leftover key of another certificate,
# is replaced by a new CA, never reused: every pair it signed is then reissued too.
postgres_ca_is_current() {
    [ -s "$POSTGRES_SIGNER_DIR/pgca.crt" ] \
        && [ -s "$POSTGRES_SIGNER_DIR/pgca.key" ] \
        && openssl x509 -checkend 604800 -noout -in "$POSTGRES_SIGNER_DIR/pgca.crt" >/dev/null 2>&1 \
        && pair_matches "$POSTGRES_SIGNER_DIR/pgca.crt" "$POSTGRES_SIGNER_DIR/pgca.key" \
        && postgres_ca_is_dedicated "$POSTGRES_SIGNER_DIR/pgca.crt"
}

# The CA certificate is the dedicated PostgreSQL CA, not the stack CA of an earlier deployment.
postgres_ca_is_dedicated() {
    openssl x509 -noout -subject -in "$1" 2>/dev/null | grep -q "CN *= *$PG_CA_SUBJECT\$"
}

issue_postgres_ca() {
    mkdir -p "$POSTGRES_SIGNER_DIR"
    chmod 0700 "$POSTGRES_SIGNER_DIR"
    rm -f "$POSTGRES_SIGNER_DIR"/pgca.* "$POSTGRES_SIGNER_DIR"/*.srl
    openssl genrsa -out "$POSTGRES_SIGNER_DIR/pgca.key" 4096
    openssl req -x509 -new -sha256 -days 30 \
        -key "$POSTGRES_SIGNER_DIR/pgca.key" \
        -subj "/CN=$PG_CA_SUBJECT" \
        -out "$POSTGRES_SIGNER_DIR/pgca.crt"
    chmod 0400 "$POSTGRES_SIGNER_DIR/pgca.key"
    chmod 0444 "$POSTGRES_SIGNER_DIR/pgca.crt"
}

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
        && [ -s "$PG_CA_CERT" ] \
        && postgres_ca_is_dedicated "$PG_CA_CERT" \
        && openssl x509 -checkend 604800 -noout -in "$PG_CA_CERT" >/dev/null 2>&1 \
        && cmp -s "$PG_CA_CERT" "$POSTGRES_TLS_DIR/ca.crt" \
        && { [ -z "$POSTGRES_CA_DIR" ] || cmp -s "$PG_CA_CERT" "$POSTGRES_CA_DIR/ca.crt"; } \
        && openssl x509 -checkend 604800 -noout -in "$POSTGRES_TLS_DIR/server.crt" >/dev/null 2>&1 \
        && openssl verify -CAfile "$PG_CA_CERT" "$POSTGRES_TLS_DIR/server.crt" >/dev/null 2>&1 \
        && postgres_pair_matches \
        && postgres_key_is_protected
}

# The deploy fast path (scripts/lib.sh) runs this same predicate inside the running postgres container, where
# only the volume it serves is visible: `sh -s -- --check-postgres-pair < init.sh` with POSTGRES_TLS_DIR set
# to that volume. The PostgreSQL CA certificate is then the CA copy next to the server pair (it must still be
# the dedicated CA and verify the certificate); a CA rotated since is caught because tls-init, which a full
# deploy always runs, reissues the pair and both copies in the same run.
if [ "${1:-}" = --check-postgres-pair ]; then
    PG_CA_CERT="$POSTGRES_TLS_DIR/ca.crt"
    POSTGRES_CA_DIR=''
    postgres_certificate_is_current
    exit $?
fi

if [ -n "$POSTGRES_TLS_DIR" ]; then
    [ -n "$POSTGRES_SIGNER_DIR" ] || { echo "POSTGRES_SIGNER_DIR is required with POSTGRES_TLS_DIR." >&2; exit 1; }
    PG_CA_CERT="$(postgres_ca_cert)"
fi
mkdir -p "$TLS_DIR"

# issue_certificate NAME [WORKDIR [CA_CERT CA_KEY]]: signs with the stack CA unless a CA pair is given.
issue_certificate() {
    name="$1"
    work="${2:-$TLS_DIR}"
    signing_cert="${3:-$TLS_DIR/ca.crt}"
    signing_key="${4:-$TLS_DIR/ca.key}"

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
        -CA "$signing_cert" \
        -CAkey "$signing_key" \
        -CAserial "$work/$name.srl" \
        -CAcreateserial \
        -extfile "$work/$name.ext" \
        -out "$work/$name.crt"
}

# Issues the postgres server pair from the PostgreSQL CA (key in the signer-only volume) and keeps it out of TLS_DIR, so no service that
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
    issue_certificate postgres "$work" "$PG_CA_CERT" "$POSTGRES_SIGNER_DIR/pgca.key"
    cp "$PG_CA_CERT" "$work/ca.crt"
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
        cp "$PG_CA_CERT" "$POSTGRES_CA_DIR/.ca.crt.new"
        chmod 0444 "$POSTGRES_CA_DIR/.ca.crt.new"
        mv "$POSTGRES_CA_DIR/.ca.crt.new" "$POSTGRES_CA_DIR/ca.crt"
    fi
    rm -f "$TLS_DIR"/*.srl "$POSTGRES_SIGNER_DIR"/*.srl
}

# A PostgreSQL CA that is missing, due or inconsistent is replaced by a new one (never reused), and then no
# pair or CA copy matches it, so everything it signs is reissued in the same run.
ensure_postgres_tls() {
    [ -n "$POSTGRES_TLS_DIR" ] || return 0
    postgres_ca_is_current || issue_postgres_ca
    postgres_certificate_is_current || issue_postgres_certificate
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
    ensure_postgres_tls
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

ensure_postgres_tls
