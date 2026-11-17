-- Release recovery addresses retained by accounts finalized before this cleanup.
DELETE FROM account_backup_emails AS backup
USING accounts AS owner
WHERE backup.account_id = owner.id AND owner.status = 'closed';
