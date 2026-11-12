-- Registry route records for native admission grants (gateway login contract
-- §7.3, §17 mode 34a), testing/preproduction only. One row per Registry-issued
-- (world, channel) scope. A scope without a complete route record, or with
-- native_login_enabled false, routes nowhere. The issuer recomputes
-- native_route_revision from the descriptor and refuses a stored value that
-- differs. No production route record is authorized by this table.
CREATE TABLE game_channels (
    world_id uuid NOT NULL CHECK (world_id::text ~ '^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$'),
    channel_id uuid NOT NULL CHECK (channel_id::text ~ '^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$'),
    native_route_host text CHECK (octet_length(native_route_host) BETWEEN 1 AND 253),
    native_route_port integer CHECK (native_route_port BETWEEN 1 AND 65535),
    native_route_tls_server_name text CHECK (octet_length(native_route_tls_server_name) BETWEEN 1 AND 253),
    native_route_version bigint CHECK (native_route_version > 0),
    native_route_revision text CHECK (octet_length(native_route_revision) BETWEEN 1 AND 64),
    native_login_enabled boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (world_id, channel_id),
    CHECK (channel_id <> world_id),
    CHECK (
        (native_route_host IS NULL AND native_route_port IS NULL AND native_route_tls_server_name IS NULL
            AND native_route_version IS NULL AND native_route_revision IS NULL)
        OR (native_route_host IS NOT NULL AND native_route_port IS NOT NULL AND native_route_tls_server_name IS NOT NULL
            AND native_route_version IS NOT NULL AND native_route_revision IS NOT NULL)
    )
);
