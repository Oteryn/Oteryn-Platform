<?php

namespace App\GameAuth\NativeLogin;

use App\Audit\SecurityEventRecorder;
use App\GameAuth\Tickets\GameLoginTicket;
use App\GameAuth\Tickets\GameLoginTicketSecrets;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use LogicException;
use SensitiveParameter;

/**
 * Native Game Login Ticket redemption (contract §4.2). A native ticket has audience
 * oteryn-native-game-gateway, the canonical AccountId and native_security_generation of its
 * Identity and no Canary binding; issuing it through an OAuth client waits for U1 (§4.1).
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
