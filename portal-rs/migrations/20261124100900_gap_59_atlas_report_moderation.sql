-- Keep Atlas intake and moderation on the same row, including existing reports.
ALTER TABLE atlas_map_reports
    ADD COLUMN lock_version bigint NOT NULL DEFAULT 1 CHECK (lock_version > 0),
    ADD COLUMN public_outcome text,
    ADD COLUMN moderator_notes text,
    ADD COLUMN assigned_to uuid REFERENCES accounts(id) ON DELETE SET NULL,
    ADD COLUMN processed_at timestamptz,
    ADD COLUMN moderation_phase text CHECK (
        (moderation_phase = 'reviewing' AND status = 'pending') OR
        (moderation_phase = 'closed' AND status IN ('resolved', 'rejected'))
    );

CREATE INDEX atlas_map_reports_queue_created ON atlas_map_reports (created_at DESC, id DESC);

CREATE VIEW support_report_queue AS
    SELECT id, reporter_account_id, report_type, category, target_reference,
           evidence_summary, status, public_outcome, moderator_notes, lock_version,
           created_at, processed_at
    FROM player_reports
    UNION ALL
    SELECT id, account_id, 'atlas', 'map-error',
           format('Atlas · (%s, %s, %s)', x, y, floor), description,
           coalesce(moderation_phase, CASE status
               WHEN 'pending' THEN 'submitted'
               WHEN 'resolved' THEN 'actioned'
               ELSE status
           END),
           public_outcome, moderator_notes, lock_version, created_at, processed_at
    FROM atlas_map_reports;
