<?php

namespace App\Identity\Actions;

use App\Audit\SecurityEventRecorder;
use App\GameAuth\OAuth\NativeOAuthGenerationBinding;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use LogicException;

final class RevokeIdentityGameAuthorizations
{
    public function __construct(
        private readonly SecurityEventRecorder $securityEvents,
        private readonly NativeOAuthGenerationBinding $oauthBindings,
    ) {}

    public function execute(Identity $identity): int
    {
        return DB::transaction(function () use ($identity): int {
            $lockedIdentity = Identity::query()
                ->whereKey($identity->id)
                ->lockForUpdate()
                ->first();

            if (! $lockedIdentity instanceof Identity) {
                throw new LogicException('Identity is unavailable for game authorization revocation.');
            }

            $generation = $lockedIdentity->game_auth_generation + 1;
            $lockedIdentity->forceFill(['game_auth_generation' => $generation])->save();

            $this->oauthBindings->revokeForIdentity($lockedIdentity);
            $this->securityEvents->recordIdentityGameAuthorizationsRevoked($lockedIdentity->id);

            $identity->setAttribute('game_auth_generation', $generation);

            return $generation;
        });
    }
}
