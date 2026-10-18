<?php

namespace App\GameAuth\DeviceSessions;

use App\GameAuth\OAuth\NativeOAuthClientManager;
use App\GameAuth\OAuth\OAuthBootstrapDenied;
use App\GameAuth\OAuth\VerifyNativeOAuthAccess;
use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalAccountId;
use Carbon\CarbonImmutable;
use Closure;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use Laravel\Passport\Client;
use LogicException;
use SensitiveParameter;

/**
 * Candidate native-only remembered-device authority. Not an OAuth refresh grant.
 * No route or default activation is introduced by this service.
 */
final class NativeRememberedDeviceSessions
{
    public function __construct(
        private readonly VerifyNativeOAuthAccess $oauth,
        private readonly NativeOAuthClientManager $clients,
        private readonly DeviceSessionSecrets $secrets,
    ) {}

    public function enroll(Identity $identity, string $accessTokenId, bool $explicitConsent = false): IssuedDeviceSessionCredential
    {
        $this->assertEnabled();
        $this->assertStandaloneTransaction();
        if (! $explicitConsent) {
            throw new DeviceSessionDenied;
        }

        return DB::transaction(function () use ($identity, $accessTokenId): IssuedDeviceSessionCredential {
            try {
                $verified = $this->oauth->locked($identity, $accessTokenId);
            } catch (OAuthBootstrapDenied) {
                throw new DeviceSessionDenied;
            }

            // Canary/legacy clients can never acquire this separate native authority.
            if (! $verified->nativeTicket || ! $this->usable($verified->identity)
                || DeviceSessionFamily::query()->where('enrollment_access_token_id', $accessTokenId)->exists()) {
                throw new DeviceSessionDenied;
            }

            $now = CarbonImmutable::now();
            $absoluteExpiry = $now->addSeconds($this->absoluteTtl());
            $family = new DeviceSessionFamily;
            $family->forceFill([
                'id' => (string) Str::uuid(),
                'identity_id' => $verified->identity->id,
                'account_id' => $verified->identity->account_id,
                'oauth_client_id' => (string) $verified->token->client_id,
                'enrollment_access_token_id' => $accessTokenId,
                'game_auth_generation' => $verified->identity->game_auth_generation,
                'native_security_generation' => $verified->identity->native_security_generation,
                'current_sequence' => 1,
                'absolute_expires_at' => $absoluteExpiry,
                'idle_expires_at' => $this->idleExpiry($now, $absoluteExpiry),
                'revoked_at' => null,
                'revocation_reason' => null,
            ])->save();

            return $this->issueCredential($family, $now);
        }, attempts: 3);
    }

    /**
     * One locked operation consumes this credential and returns its sole successor.
     * The native application callback runs within the same transaction as rotation.
     * Denial is thrown AFTER commit so replay/expiry/security revocation is durable.
     *
     * @template TResult
     *
     * @param  Closure(VerifiedRememberedDeviceAuthorization): TResult  $consume
     * @return RotatedDeviceSessionResult<TResult>
     */
    public function rotateAndUse(
        #[SensitiveParameter] string $secret,
        string $oauthClientId,
        DeviceSessionPurpose $purpose,
        Closure $consume,
    ): RotatedDeviceSessionResult {
        $this->assertEnabled();
        $this->assertStandaloneTransaction();
        if (! $this->secrets->valid($secret)) {
            throw new DeviceSessionDenied;
        }

        $result = DB::transaction(function () use ($secret, $oauthClientId, $purpose, $consume): ?RotatedDeviceSessionResult {
            $locked = $this->lockedCredential($secret, $oauthClientId);
            if ($locked === null) {
                return null;
            }
            [$identity, $client, $family, $credential] = $locked;
            if ($family->revoked_at !== null) {
                return null;
            }
            if ($credential->consumed_at !== null || $credential->sequence !== $family->current_sequence) {
                $this->revokeFamily($family, 'credential_replay');

                return null;
            }

            $now = CarbonImmutable::now();
            if ($family->absolute_expires_at->lte($now) || $family->idle_expires_at->lte($now)) {
                $this->revokeFamily($family, 'expired');

                return null;
            }
            if (! $this->authorizationCurrent($identity, $client, $family)) {
                $this->revokeFamily($family, 'authorization_changed');

                return null;
            }
            if ($family->current_sequence < 1 || $family->current_sequence >= PHP_INT_MAX) {
                $this->revokeFamily($family, 'rotation_exhausted');

                return null;
            }

            $credential->forceFill(['consumed_at' => $now])->save();
            $family->forceFill([
                'current_sequence' => $family->current_sequence + 1,
                'idle_expires_at' => $this->idleExpiry($now, $family->absolute_expires_at),
            ])->save();
            $successor = $this->issueCredential($family, $now);
            $authorization = new VerifiedRememberedDeviceAuthorization(
                $identity,
                $family->identity_id,
                $family->account_id,
                $family->oauth_client_id,
                $family->game_auth_generation,
                $family->native_security_generation,
                $family->id,
                $purpose,
            );

            // No external calls, notifications, queues, or other non-transactional effects here.
            // A callback failure rolls back the token consumption and application mutation.
            return new RotatedDeviceSessionResult($successor, $consume($authorization));
        }, attempts: 3);

        if ($result === null) {
            throw new DeviceSessionDenied;
        }

        return $result;
    }

    /** A current OR already-used family credential may revoke only its own client-bound family. */
    public function revokeCredential(#[SensitiveParameter] string $secret, string $oauthClientId): void
    {
        $this->assertEnabled();
        $this->assertStandaloneTransaction();
        if (! $this->secrets->valid($secret)) {
            throw new DeviceSessionDenied;
        }

        $accepted = DB::transaction(function () use ($secret, $oauthClientId): bool {
            $locked = $this->lockedCredential($secret, $oauthClientId);
            if ($locked === null) {
                return false;
            }
            $this->revokeFamily($locked[2], 'client_revoked');

            return true;
        }, attempts: 3);
        if (! $accepted) {
            throw new DeviceSessionDenied;
        }
    }

    /** Browser account management must authenticate and CSRF-protect the owner before this call. */
    public function revokeForOwner(Identity $identity, string $familyId): void
    {
        $this->assertEnabled();
        $this->assertStandaloneTransaction();
        $accepted = DB::transaction(function () use ($identity, $familyId): bool {
            $lockedIdentity = Identity::query()->whereKey($identity->id)->lockForUpdate()->first();
            if (! $lockedIdentity instanceof Identity || ! $this->usable($lockedIdentity)) {
                return false;
            }
            $family = DeviceSessionFamily::query()->whereKey($familyId)
                ->where('identity_id', $lockedIdentity->id)->lockForUpdate()->first();
            if (! $family instanceof DeviceSessionFamily) {
                return false;
            }
            $this->revokeFamily($family, 'owner_revoked');

            return true;
        }, attempts: 3);
        if (! $accepted) {
            throw new DeviceSessionDenied;
        }
    }

    /**
     * Locator reads carry no authority. Locks consistently start with Identity, preventing
     * a refresh/revocation race from resurrecting an old generation or deadlocking enrollment.
     *
     * @return array{Identity, Client|null, DeviceSessionFamily, DeviceSessionCredential}|null
     */
    private function lockedCredential(#[SensitiveParameter] string $secret, string $oauthClientId): ?array
    {
        $hash = $this->secrets->hash($secret);
        $locator = DeviceSessionCredential::query()->whereKey($hash)->first();
        if (! $locator instanceof DeviceSessionCredential) {
            return null;
        }
        $familyLocator = DeviceSessionFamily::query()->whereKey($locator->family_id)->first();
        if (! $familyLocator instanceof DeviceSessionFamily
            || ! hash_equals($familyLocator->oauth_client_id, $oauthClientId)) {
            return null;
        }

        $identity = Identity::query()->whereKey($familyLocator->identity_id)->lockForUpdate()->first();
        if (! $identity instanceof Identity) {
            return null;
        }
        $client = Client::query()->whereKey($oauthClientId)->lockForUpdate()->first();
        $family = DeviceSessionFamily::query()->whereKey($locator->family_id)->lockForUpdate()->first();
        $credential = DeviceSessionCredential::query()->whereKey($hash)->lockForUpdate()->first();
        if (! $family instanceof DeviceSessionFamily || ! $credential instanceof DeviceSessionCredential
            || $family->identity_id !== $identity->id
            || ! hash_equals($family->oauth_client_id, $oauthClientId)
            || ! hash_equals($credential->family_id, $family->id)) {
            return null;
        }

        return [$identity, $client, $family, $credential];
    }

    private function authorizationCurrent(Identity $identity, ?Client $client, DeviceSessionFamily $family): bool
    {
        if (! $this->usable($identity)
            || ! hash_equals($identity->account_id, $family->account_id)
            || $identity->game_auth_generation !== $family->game_auth_generation
            || $identity->native_security_generation !== $family->native_security_generation
            || ! $client instanceof Client) {
            return false;
        }

        try {
            return $this->clients->usesNativeTicket($client);
        } catch (LogicException) {
            return false;
        }
    }

    private function usable(Identity $identity): bool
    {
        return $identity->disabled_at === null && ! $identity->isTerminated()
            && CanonicalAccountId::isValid($identity->account_id)
            && $identity->game_auth_generation >= 0 && $identity->native_security_generation >= 1;
    }

    private function issueCredential(DeviceSessionFamily $family, CarbonImmutable $now): IssuedDeviceSessionCredential
    {
        $secret = $this->secrets->generate();
        $credential = new DeviceSessionCredential;
        $credential->forceFill([
            'token_hash' => $this->secrets->hash($secret),
            'family_id' => $family->id,
            'sequence' => $family->current_sequence,
            'issued_at' => $now,
            'consumed_at' => null,
        ])->save();

        return new IssuedDeviceSessionCredential($secret, $family->id, $family->absolute_expires_at, $family->idle_expires_at);
    }

    private function revokeFamily(DeviceSessionFamily $family, string $reason): void
    {
        if ($family->revoked_at === null) {
            $family->forceFill(['revoked_at' => CarbonImmutable::now(), 'revocation_reason' => $reason])->save();
        }
    }

    private function assertEnabled(): void
    {
        if (! app()->environment(['testing', 'preproduction'])
            || config('game-auth.device_sessions.enabled', false) !== true) {
            throw new DeviceSessionDenied;
        }

        $passportConnection = config('passport.connection');
        if ($passportConnection !== null && $passportConnection !== DB::getDefaultConnection()) {
            throw new LogicException('Native remembered-device authority requires Identity, OAuth and family tables on one database connection.');
        }
    }

    private function assertStandaloneTransaction(): void
    {
        if (DB::transactionLevel() !== 0) {
            throw new LogicException('Remembered-device operations own their transaction so terminal replay revocation cannot be rolled back by a caller.');
        }
    }

    private function absoluteTtl(): int
    {
        return $this->boundedTtl('game-auth.device_sessions.absolute_ttl_seconds', 2592000);
    }

    private function idleTtl(): int
    {
        return $this->boundedTtl('game-auth.device_sessions.idle_ttl_seconds', 604800);
    }

    private function boundedTtl(string $key, int $maximum): int
    {
        $ttl = config($key, $maximum);
        if (! is_int($ttl) || $ttl < 1 || $ttl > $maximum) {
            throw new LogicException('Native remembered-device lifetime configuration is invalid.');
        }

        return $ttl;
    }

    private function idleExpiry(CarbonImmutable $now, CarbonImmutable $absoluteExpiry): CarbonImmutable
    {
        $idleExpiry = $now->addSeconds($this->idleTtl());

        return $idleExpiry->lt($absoluteExpiry) ? $idleExpiry : $absoluteExpiry;
    }
}
