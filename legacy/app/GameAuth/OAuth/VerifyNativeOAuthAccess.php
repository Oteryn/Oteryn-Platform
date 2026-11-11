<?php

namespace App\GameAuth\OAuth;

use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Laravel\Passport\Client;
use Laravel\Passport\Token;
use LogicException;

final class VerifyNativeOAuthAccess
{
    public function __construct(private readonly NativeOAuthClientManager $nativeClients) {}

    /**
     * Lock and verify the first-party native bearer policy used by both owner reads and ticket bootstrap.
     * The caller owns the surrounding transaction so verified authorization facts are consumed atomically.
     */
    public function locked(Identity $identity, string $accessTokenId): VerifiedNativeOAuthAccess
    {
        if (DB::transactionLevel() < 1) {
            throw new LogicException('Native OAuth access verification requires a transaction.');
        }

        $lockedIdentity = Identity::query()
            ->whereKey($identity->id)
            ->lockForUpdate()
            ->first();
        if (! $lockedIdentity instanceof Identity
            || $lockedIdentity->disabled_at !== null
            || $lockedIdentity->isTerminated()) {
            throw new OAuthBootstrapDenied;
        }

        $accessToken = Token::query()
            ->whereKey($accessTokenId)
            ->lockForUpdate()
            ->first();

        $tokenUserId = $accessToken?->getAttribute('user_id');
        $identityId = $lockedIdentity->getAuthIdentifier();
        $tokenGeneration = $this->generation($accessToken?->getAttribute('game_auth_generation'));

        if (! $accessToken instanceof Token
            || (! is_int($tokenUserId) && ! is_string($tokenUserId))
            || (! is_int($identityId) && ! is_string($identityId))
            || $accessToken->revoked
            || $accessToken->expires_at === null
            || $accessToken->expires_at->lte(now())
            || (string) $tokenUserId !== (string) $identityId
            || $tokenGeneration === null
            || $tokenGeneration !== $lockedIdentity->game_auth_generation
            || ! $accessToken->can('game:ticket')) {
            throw new OAuthBootstrapDenied;
        }

        $client = Client::query()
            ->whereKey($accessToken->client_id)
            ->lockForUpdate()
            ->first();
        if (! $client instanceof Client) {
            throw new OAuthBootstrapDenied;
        }

        try {
            $nativeTicket = $this->nativeClients->usesNativeTicket($client);
        } catch (LogicException) {
            throw new OAuthBootstrapDenied;
        }

        return new VerifiedNativeOAuthAccess($lockedIdentity, $accessToken, $nativeTicket);
    }

    private function generation(mixed $generation): ?int
    {
        if (is_int($generation) && $generation >= 0) {
            return $generation;
        }
        if (is_string($generation) && ctype_digit($generation)) {
            return (int) $generation;
        }

        return null;
    }
}
