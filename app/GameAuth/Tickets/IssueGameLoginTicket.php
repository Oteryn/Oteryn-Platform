<?php

namespace App\GameAuth\Tickets;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use LogicException;

final class IssueGameLoginTicket
{
    public function __construct(
        private readonly GameLoginTicketSecrets $secrets,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(Identity $identity): IssuedGameLoginTicket
    {
        if (config('game-auth.legacy_login.enabled', true) !== true) {
            throw new GameLoginTicketDenied;
        }

        return DB::transaction(function () use ($identity): IssuedGameLoginTicket {
            $lockedIdentity = Identity::query()
                ->lockForUpdate()
                ->find($identity->id);

            if (! $lockedIdentity instanceof Identity || $lockedIdentity->disabled_at !== null) {
                throw new GameLoginTicketDenied;
            }

            $binding = IdentityCanaryAccount::query()
                ->lockForUpdate()
                ->find($lockedIdentity->id);

            if (! $binding instanceof IdentityCanaryAccount || ! $binding->isReady()) {
                throw new GameLoginTicketDenied;
            }

            $ttlSeconds = config('game-auth.ticket.ttl_seconds', 60);
            $audience = config('game-auth.ticket.audience', '');

            if (! is_int($ttlSeconds)
                || $ttlSeconds < 1
                || $ttlSeconds > 300
                || ! is_string($audience)
                || $audience === ''
            ) {
                throw new LogicException('Invalid game authentication ticket configuration.');
            }

            $ticket = $this->secrets->generate();
            $expiresAt = now()->addSeconds($ttlSeconds);

            GameLoginTicket::query()->create([
                'ticket_hash' => $this->secrets->hash($ticket),
                'identity_id' => $lockedIdentity->id,
                'canary_account_id' => $binding->canary_account_id,
                'audience' => $audience,
                'security_generation' => $lockedIdentity->game_auth_generation,
                'expires_at' => $expiresAt,
            ]);

            $this->securityEvents->recordGameLoginTicketIssued($lockedIdentity->id);

            return new IssuedGameLoginTicket($ticket, $expiresAt);
        });
    }
}
