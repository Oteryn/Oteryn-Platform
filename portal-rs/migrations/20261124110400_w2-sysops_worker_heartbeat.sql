CREATE TABLE portal_worker_heartbeat (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    seen_at timestamptz NOT NULL
);

-- Extend the installed permission catalog, including any earlier additions.
DO $$
DECLARE
    existing_check text;
BEGIN
    SELECT pg_get_expr(conbin, conrelid) INTO STRICT existing_check
    FROM pg_constraint
    WHERE conrelid = 'role_permissions'::regclass
      AND conname = 'role_permissions_permission_check'
      AND contype = 'c';

    ALTER TABLE role_permissions DROP CONSTRAINT role_permissions_permission_check;
    EXECUTE format(
        'ALTER TABLE role_permissions ADD CONSTRAINT role_permissions_permission_check CHECK ((%s) OR permission = %L)',
        existing_check, 'admin.system.view'
    );
END;
$$;

INSERT INTO role_permissions (role_id, permission)
SELECT id, 'admin.system.view' FROM roles
WHERE key = 'platform_admin' AND built_in
ON CONFLICT DO NOTHING;
