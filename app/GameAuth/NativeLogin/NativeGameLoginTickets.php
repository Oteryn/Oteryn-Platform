<?php

namespace App\GameAuth\NativeLogin;

use App\Audit\SecurityEventRecorder;
use App\GameAuth\Tickets\GameLoginTicket;
use App\GameAuth\Tickets\GameLoginTicketDenied;
use App\GameAuth\Tickets\GameLoginTicketSecrets;
use App\GameAuth\Tickets\IssuedGameLoginTicket;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use LogicException;
use SensitiveParameter;

/**
 * Native Game Login Ticket redemption (contract §4.2). A native ticket has audience
 * oteryn-native-game-gateway, the canonical AccountId and native_security_generation of its
 * Identity and no Canary binding; the separate first-party Rust OAuth client selects issuance.
 * Redemption yields the AccountId and runs only inside the issuer transaction (§6.2).
 */
final class NativeGameLoginTickets
{
    public const AUDIENCE = 'oteryn-native-game-gateway';

    private const UUID_V7 = '/\A[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/';

    public function __construct(
        private readonly GameLoginTicketSecrets $secrets,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function hash(#[SensitiveParameter] string $ticket): string
    {
        return $this->secrets->hash($ticket);
    }

    public function issue(Identity $identity): IssuedGameLoginTicket
    {
        if (DB::transactionLevel() < 1) {
            throw new LogicException('Native ticket issuance must run inside the OAuth transaction.');
        }
        if (! app()->environment(['testing', 'preproduction'])
            || config('game-auth.native_admission.enabled') !== true) {
            throw new GameLoginTicketDenied;
        }
        $lockedIdentity = Identity::query()->lockForUpdate()->find($identity->id);
        if (! $lockedIdentity instanceof Identity || ! self::usable($lockedIdentity)) {
            throw new GameLoginTicketDenied;
        }
        $ttl = config('game-auth.ticket.ttl_seconds', 60);
        if (! is_int($ttl) || $ttl < 1 || $ttl > 60) {
            throw new LogicException('Native ticket TTL must be between one and sixty seconds.');
        }
        $ticket = $this->secrets->generate();
        $expiresAt = now()->addSeconds($ttl);
        GameLoginTicket::query()->create([
            'ticket_hash' => $this->secrets->hash($ticket),
            'identity_id' => $lockedIdentity->id,
            'canary_account_id' => null,
            'account_id' => $lockedIdentity->account_id,
            'audience' => self::AUDIENCE,
            'security_generation' => $lockedIdentity->game_auth_generation,
            'native_security_generation' => $lockedIdentity->native_security_generation,
            'expires_at' => $expiresAt,
        ]);
        $this->securityEvents->recordGameLoginTicketIssued($lockedIdentity->id);

        return new IssuedGameLoginTicket($ticket, $expiresAt);
    }

    /**
     * Contract §4.2 steps 1-7 under row locks. Must run inside the caller's transaction so a later
     * rejection rolls the redemption back. A ticket already consumed by this same attempt_ref is
     * reported as NativeAdmissionAttemptRace so the caller re-reads the committed attempt.
     */
    public function redeem(#[SensitiveParameter] string $ticket, string $attemptRef): RedeemedNativeAccount
    {
        if (DB::transactionLevel() < 1) {
            throw new LogicException('Native ticket redemption must run inside the issuer transaction.');
        }

        $stored = GameLoginTicket::query()
            ->where('ticket_hash', $this->secrets->hash($ticket))
            ->lockForUpdate()
            ->first();
        if (! $stored instanceof GameLoginTicket || ! hash_equals(self::AUDIENCE, $stored->audience)) {
            throw new NativeLoginRefused(NativeLoginError::TicketRejected);
        }
        if ($stored->used_at !== null) {
            if ($stored->attempt_ref !== null && hash_equals($stored->attempt_ref, $attemptRef)) {
                throw new NativeAdmissionAttemptRace;
            }
            throw new NativeLoginRefused(NativeLoginError::TicketRejected);
        }
        $accountId = $stored->account_id;
        $nativeGeneration = $stored->native_security_generation;
        if ($stored->expires_at->lte(now()) || $stored->canary_account_id !== null
            || $accountId === null || $nativeGeneration === null) {
            throw new NativeLoginRefused(NativeLoginError::TicketRejected);
        }

        $identity = Identity::query()->lockForUpdate()->find($stored->identity_id);
        if (! $identity instanceof Identity || ! self::usable($identity)
            || ! hash_equals($identity->account_id, $accountId)
            || $identity->native_security_generation !== $nativeGeneration
            || $identity->game_auth_generation !== $stored->security_generation) {
            throw new NativeLoginRefused(NativeLoginError::AccountSecurityDenied);
        }

        $stored->forceFill(['used_at' => now(), 'attempt_ref' => $attemptRef])->save();
        $this->securityEvents->recordGameLoginTicketRedeemed($identity->id);

        return new RedeemedNativeAccount(
            $identity->id,
            $identity->account_id,
            (string) $identity->native_security_generation,
        );
    }

    private static function usable(Identity $identity): bool
    {
        return $identity->disabled_at === null
            && $identity->terminated_at === null
            && preg_match(self::UUID_V7, $identity->account_id) === 1
            && $identity->native_security_generation >= 1;
    }
}
