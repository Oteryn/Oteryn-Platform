<?php

namespace App\Audit;

use Illuminate\Support\Facades\DB;

final class SecurityEventRecorder
{
    public const IDENTITY_REGISTERED = 'identity.registered';

    public const IDENTITY_LOGIN_SUCCEEDED = 'identity.login_succeeded';

    public const IDENTITY_LOGGED_OUT = 'identity.logged_out';

    public const IDENTITY_WEB_SESSIONS_REVOKED = 'identity.web_sessions_revoked';

    public const IDENTITY_WEB_SESSION_REVOKED = 'identity.web_session_revoked';

    public const IDENTITY_OTHER_WEB_SESSIONS_REVOKED = 'identity.other_web_sessions_revoked';

    public const IDENTITY_WEB_SESSION_REJECTED = 'identity.web_session_rejected';

    public const IDENTITY_GAME_AUTHORIZATIONS_REVOKED = 'identity.game_authorizations_revoked';

    public const IDENTITY_PASSWORD_CHANGED = 'identity.password_changed';

    public const IDENTITY_PASSWORD_RESET_COMPLETED = 'identity.password_reset_completed';

    public const IDENTITY_MFA_RESET = 'identity.mfa_reset';

    public const IDENTITY_MFA_ENROLLED = 'identity.mfa_enrolled';

    public const IDENTITY_MFA_RECOVERY_CODE_USED = 'identity.mfa_recovery_code_used';

    public const IDENTITY_MFA_DISABLED = 'identity.mfa_disabled';

    public const IDENTITY_EMAIL_CHANGE_REQUESTED = 'identity.email_change_requested';

    public const IDENTITY_EMAIL_CHANGE_CONFIRMED = 'identity.email_change_confirmed';

    public const IDENTITY_EMAIL_CHANGE_CANCELLED = 'identity.email_change_cancelled';

    public const IDENTITY_EMAIL_CHANGE_RECOVERED = 'identity.email_change_recovered';

    public const IDENTITY_PRIVACY_UPDATED = 'identity.privacy_updated';

    public const IDENTITY_RECOVERY_KEY_GENERATED = 'identity.recovery_key_generated';

    public const IDENTITY_RECOVERY_KEY_REVOKED = 'identity.recovery_key_revoked';

    public const IDENTITY_RECOVERY_KEY_USED = 'identity.recovery_key_used';

    public const IDENTITY_TERMINATION_REQUESTED = 'identity.termination_requested';

    public const IDENTITY_TERMINATION_CANCELLED = 'identity.termination_cancelled';

    public const IDENTITY_TERMINATION_FINALIZED = 'identity.termination_finalized';

    public const GAME_LOGIN_TICKET_ISSUED = 'game_auth.ticket_issued';

    public const GAME_LOGIN_TICKET_REDEEMED = 'game_auth.ticket_redeemed';

    public const CANARY_ACCOUNT_PROVISIONING_REQUESTED = 'identity.canary_account_provisioning_requested';

    public const CANARY_ACCOUNT_PROVISIONING_COMPLETED = 'identity.canary_account_provisioning_completed';

    public const CANARY_ACCOUNT_PROVISIONING_FAILED = 'identity.canary_account_provisioning_failed';

    public const CANARY_ACCOUNT_PROVISIONING_CONFLICT = 'identity.canary_account_provisioning_conflict';

    public function recordIdentityRegistered(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_REGISTERED);
    }

    public function recordIdentityLoginSucceeded(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_LOGIN_SUCCEEDED);
    }

    public function recordIdentityLoggedOut(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_LOGGED_OUT);
    }

    public function recordIdentityWebSessionsRevoked(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_WEB_SESSIONS_REVOKED);
    }

    public function recordIdentityWebSessionRevoked(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_WEB_SESSION_REVOKED);
    }

    public function recordIdentityOtherWebSessionsRevoked(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_OTHER_WEB_SESSIONS_REVOKED);
    }

    public function recordIdentityWebSessionRejected(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_WEB_SESSION_REJECTED);
    }

    public function recordIdentityGameAuthorizationsRevoked(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_GAME_AUTHORIZATIONS_REVOKED);
    }

    public function recordIdentityPasswordChanged(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_PASSWORD_CHANGED);
    }

    public function recordIdentityPasswordResetCompleted(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_PASSWORD_RESET_COMPLETED);
    }

    public function recordIdentityMfaReset(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_MFA_RESET);
    }

    public function recordIdentityMfaEnrolled(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_MFA_ENROLLED);
    }

    public function recordIdentityMfaRecoveryCodeUsed(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_MFA_RECOVERY_CODE_USED);
    }

    public function recordIdentityMfaDisabled(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_MFA_DISABLED);
    }

    public function recordIdentityEmailChangeRequested(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_EMAIL_CHANGE_REQUESTED);
    }

    public function recordIdentityEmailChangeConfirmed(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_EMAIL_CHANGE_CONFIRMED);
    }

    public function recordIdentityEmailChangeCancelled(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_EMAIL_CHANGE_CANCELLED);
    }

    public function recordIdentityEmailChangeRecovered(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_EMAIL_CHANGE_RECOVERED);
    }

    public function recordIdentityPrivacyUpdated(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_PRIVACY_UPDATED);
    }

    public function recordIdentityRecoveryKeyGenerated(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_RECOVERY_KEY_GENERATED);
    }

    public function recordIdentityRecoveryKeyRevoked(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_RECOVERY_KEY_REVOKED);
    }

    public function recordIdentityRecoveryKeyUsed(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_RECOVERY_KEY_USED);
    }

    public function recordIdentityTerminationRequested(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_TERMINATION_REQUESTED);
    }

    public function recordIdentityTerminationCancelled(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_TERMINATION_CANCELLED);
    }

    public function recordIdentityTerminationFinalized(int $identityId): void
    {
        $this->record($identityId, self::IDENTITY_TERMINATION_FINALIZED);
    }

    public function recordGameLoginTicketIssued(int $identityId): void
    {
        $this->record($identityId, self::GAME_LOGIN_TICKET_ISSUED);
    }

    public function recordGameLoginTicketRedeemed(int $identityId): void
    {
        $this->record($identityId, self::GAME_LOGIN_TICKET_REDEEMED);
    }

    public function recordCanaryAccountProvisioningRequested(int $identityId): void
    {
        $this->record($identityId, self::CANARY_ACCOUNT_PROVISIONING_REQUESTED);
    }

    public function recordCanaryAccountProvisioningCompleted(int $identityId): void
    {
        $this->record($identityId, self::CANARY_ACCOUNT_PROVISIONING_COMPLETED);
    }

    public function recordCanaryAccountProvisioningFailed(int $identityId): void
    {
        $this->record($identityId, self::CANARY_ACCOUNT_PROVISIONING_FAILED);
    }

    public function recordCanaryAccountProvisioningConflict(int $identityId): void
    {
        $this->record($identityId, self::CANARY_ACCOUNT_PROVISIONING_CONFLICT);
    }

    private function record(int $identityId, string $eventType): void
    {
        DB::table('identity_security_events')->insert([
            'identity_id' => $identityId,
            'event_type' => $eventType,
            'occurred_at' => now(),
        ]);
    }
}
