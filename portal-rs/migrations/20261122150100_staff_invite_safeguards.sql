ALTER TABLE staff_invites
    ADD COLUMN request_key uuid,
    ADD COLUMN requires_approval boolean NOT NULL DEFAULT false,
    ADD COLUMN approved_by uuid REFERENCES accounts(id) ON DELETE SET NULL,
    ADD CONSTRAINT staff_invites_request UNIQUE (invited_by, request_key),
    ADD CONSTRAINT staff_invites_independent_approval CHECK (approved_by <> invited_by);

-- Pending links issued before approval was introduced still require approval
-- for the roles protected by the existing browser role-management policy.
UPDATE staff_invites i SET requires_approval=true
WHERE i.accepted_at IS NULL AND i.revoked_at IS NULL AND EXISTS (
    SELECT 1 FROM roles r WHERE r.id=ANY(i.role_ids)
    AND (r.key IN ('owner', 'platform_admin') OR EXISTS (
        SELECT 1 FROM role_permissions rp
        WHERE rp.role_id=r.id AND rp.permission='admin.roles.manage'
    ))
);

-- Existing queued work must wait for approval too. Reset the lease so an
-- in-flight worker cannot acknowledge the rescheduled job.
UPDATE outbox o
SET state='pending', leased_until=NULL, attempts=0, run_after=i.expires_at,
    updated_at=clock_timestamp()
FROM staff_invites i
WHERE o.kind='email.staff-invite' AND o.idempotency_key=i.id::text
AND i.requires_approval AND i.expires_at>clock_timestamp()
AND o.state IN ('pending','running');
