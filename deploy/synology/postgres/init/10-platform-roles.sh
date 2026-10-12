#!/bin/sh
# Platform PostgreSQL database and role split (ADR 0046 §5, docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
# P4). deploy.sh stages this file as /bootstrap/postgres-roles.sh and runs it on every full deploy inside the
# postgres container as the postgres OS user, so psql reaches the server over the local socket with peer
# authentication and no superuser password. It is idempotent: it creates what is missing and re-applies
# every attribute, password, ownership and grant, so a rotated secret takes effect on the next deploy.
#
# The four role passwords arrive on standard input, one per line, in this order: owner, app (runtime),
# backup (physical, REPLICATION), dump (logical, read-only). They never appear in a command line, an
# environment variable or the server log (statement logging is switched off for this session).
set -eu
umask 077

database=oteryn_platform
owner=oteryn_platform_owner
app=oteryn_platform_app
backup=oteryn_platform_backup
dump=oteryn_platform_dump

IFS= read -r owner_password || true
IFS= read -r app_password || true
IFS= read -r backup_password || true
IFS= read -r dump_password || true

for value in "$owner_password" "$app_password" "$backup_password" "$dump_password"; do
    case "$value" in
        '')
            echo "A PostgreSQL role password is missing; refusing the role bootstrap." >&2
            exit 1
            ;;
        *[!A-Za-z0-9_.-]*)
            echo "PostgreSQL role passwords must use only A-Z, a-z, 0-9, dot, underscore or dash." >&2
            exit 1
            ;;
    esac
    if [ "${#value}" -lt 24 ]; then
        echo "PostgreSQL role passwords must be at least 24 characters (generated hex)." >&2
        exit 1
    fi
done
if [ "$owner_password" = "$app_password" ] || [ "$owner_password" = "$backup_password" ] \
    || [ "$owner_password" = "$dump_password" ] || [ "$app_password" = "$backup_password" ] \
    || [ "$app_password" = "$dump_password" ] || [ "$backup_password" = "$dump_password" ]; then
    echo "PostgreSQL role passwords must be distinct per role." >&2
    exit 1
fi

# Cluster level: roles, the database and its CONNECT boundary. The superuser connects to the maintenance
# database; every name is a fixed identifier, every password a psql variable quoted by :'name'.
psql -X -q -v ON_ERROR_STOP=1 -d postgres <<SQL
SET log_statement = 'none';
SET log_min_error_statement = 'panic';
SET log_min_duration_statement = -1;
\set owner_password '$owner_password'
\set app_password '$app_password'
\set backup_password '$backup_password'
\set dump_password '$dump_password'
SELECT format('CREATE ROLE %I', name) FROM unnest(ARRAY['$owner', '$app', '$backup', '$dump']) AS name
    WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = name) \gexec
ALTER ROLE $owner WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS INHERIT CONNECTION LIMIT 10 PASSWORD :'owner_password';
ALTER ROLE $app WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS INHERIT CONNECTION LIMIT 30 PASSWORD :'app_password';
ALTER ROLE $backup WITH LOGIN REPLICATION NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS NOINHERIT CONNECTION LIMIT 2 PASSWORD :'backup_password';
ALTER ROLE $dump WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS INHERIT CONNECTION LIMIT 2 PASSWORD :'dump_password';
SELECT format('REVOKE %I FROM %I', g.rolname, m.rolname)
    FROM pg_auth_members am
    JOIN pg_roles g ON g.oid = am.roleid
    JOIN pg_roles m ON m.oid = am.member
    WHERE m.rolname IN ('$owner', '$app', '$backup', '$dump') \gexec
SELECT 'CREATE DATABASE $database OWNER $owner ENCODING ''UTF8'' LOCALE_PROVIDER ''libc'' LOCALE ''C.UTF-8'' TEMPLATE template0'
    WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = '$database') \gexec
ALTER DATABASE $database OWNER TO $owner;
REVOKE ALL ON DATABASE $database FROM PUBLIC;
REVOKE ALL ON DATABASE $database FROM $app, $dump, $backup;
GRANT CONNECT ON DATABASE $database TO $app, $dump;
SQL

# Database level: the owner owns schema public and every object the migrations create; the runtime role
# gets DML only, the dump role SELECT only, now and for every future object the owner creates.
psql -X -q -v ON_ERROR_STOP=1 -d "$database" <<SQL
SET log_statement = 'none';
ALTER SCHEMA public OWNER TO $owner;
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO $app, $dump;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO $app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO $app;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO $dump;
GRANT SELECT ON ALL SEQUENCES IN SCHEMA public TO $dump;
ALTER DEFAULT PRIVILEGES FOR ROLE $owner IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO $app;
ALTER DEFAULT PRIVILEGES FOR ROLE $owner IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO $app;
ALTER DEFAULT PRIVILEGES FOR ROLE $owner IN SCHEMA public GRANT SELECT ON TABLES TO $dump;
ALTER DEFAULT PRIVILEGES FOR ROLE $owner IN SCHEMA public GRANT SELECT ON SEQUENCES TO $dump;
SQL

echo "PostgreSQL roles and database $database are provisioned."
