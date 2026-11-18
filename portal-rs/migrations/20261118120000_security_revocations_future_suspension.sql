-- Preserve whether an approval preceded the effective window. An approval
-- first written after expiry must never cause a session revocation.
ALTER TABLE sanctions ADD COLUMN security_revocation_scheduled boolean NOT NULL DEFAULT false;

CREATE FUNCTION identity_security_revocation_track_schedule() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' OR NEW.status IS DISTINCT FROM OLD.status
       OR NEW.category IS DISTINCT FROM OLD.category OR NEW.effective_at IS DISTINCT FROM OLD.effective_at THEN
        NEW.security_revocation_scheduled := NEW.category = 'suspension' AND NEW.status = 'active'
            AND NEW.effective_at > clock_timestamp();
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER identity_security_revocation_track_schedule
    BEFORE INSERT OR UPDATE OF status, category, effective_at ON sanctions
    FOR EACH ROW EXECUTE FUNCTION identity_security_revocation_track_schedule();

CREATE OR REPLACE FUNCTION identity_security_revocation_reconcile() RETURNS void LANGUAGE plpgsql AS $$
DECLARE item record;
BEGIN
    PERFORM 1 FROM identity_security_revocation_counter WHERE singleton FOR UPDATE SKIP LOCKED;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    FOR item IN
        SELECT s.id, s.account_id, a.native_security_generation AS generation
        FROM sanctions s JOIN accounts a ON a.id=s.account_id
        WHERE s.category='suspension' AND s.status='active'
            AND s.effective_at <= clock_timestamp()
            AND (s.security_revocation_scheduled OR s.expires_at IS NULL OR s.expires_at > clock_timestamp())
            AND NOT EXISTS (SELECT 1 FROM identity_security_revocation_suspensions m
                WHERE m.sanction_id=s.id)
        ORDER BY s.effective_at, s.id LIMIT 500
    LOOP
        PERFORM identity_security_revocation_append(item.account_id,item.generation,3::smallint);
        INSERT INTO identity_security_revocation_suspensions(sanction_id) VALUES (item.id);
    END LOOP;
END $$;
