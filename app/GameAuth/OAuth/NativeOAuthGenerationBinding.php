<?php

namespace App\GameAuth\OAuth;

use App\Identity\Models\Identity;
use Defuse\Crypto\Crypto;
use Illuminate\Contracts\Container\Container;
use Illuminate\Contracts\Encryption\Encrypter;
use Illuminate\Http\Request;
use Laravel\Passport\AuthCode;
use Laravel\Passport\Events\AccessTokenCreated;
use Laravel\Passport\Passport;
use Laravel\Passport\RefreshToken;
use Laravel\Passport\Token;
use LogicException;
use Throwable;

final class NativeOAuthGenerationBinding
{
    public function __construct(
        private readonly Encrypter $encrypter,
        private readonly Container $container,
    ) {}

    public function bindAuthorizationCode(AuthCode $authCode): void
    {
        if (! $this->authCodeHasGameTicketScope($authCode)) {
            return;
        }

        $identity = Identity::query()->find($authCode->getAttribute('user_id'));

        if (! $identity instanceof Identity) {
            throw new LogicException('Native OAuth authorization identity is unavailable.');
        }

        $authCode->setAttribute('game_auth_generation', $this->identityGeneration($identity));
    }

    public function bindAccessToken(AccessTokenCreated $event): void
    {
        $accessToken = Token::query()->find($event->tokenId);

        if (! $accessToken instanceof Token || ! $accessToken->can('game:ticket')) {
            return;
        }

        try {
            $generation = $this->sourceGeneration();
            $identity = Identity::query()->find($event->userId);

            if (! $identity instanceof Identity
                || $generation !== $this->identityGeneration($identity)
            ) {
                throw new LogicException('Native OAuth authorization generation is no longer current.');
            }

            $accessToken->forceFill(['game_auth_generation' => $generation])->save();
        } catch (Throwable $exception) {
            $accessToken->forceFill(['revoked' => true])->save();

            if ($exception instanceof LogicException) {
                throw $exception;
            }

            throw new LogicException('Native OAuth authorization generation could not be bound.', 0, $exception);
        }
    }

    public function revokeForIdentity(Identity $identity): void
    {
        $authCodes = AuthCode::query()
            ->where('user_id', $identity->getAuthIdentifier())
            ->where('revoked', false)
            ->lockForUpdate()
            ->get();

        foreach ($authCodes as $authCode) {
            if ($authCode instanceof AuthCode && $this->authCodeHasGameTicketScope($authCode)) {
                $authCode->forceFill(['revoked' => true])->save();
            }
        }

        $accessTokens = Token::query()
            ->where('user_id', $identity->getAuthIdentifier())
            ->lockForUpdate()
            ->get()
            ->filter(static fn (Token $token): bool => $token->can('game:ticket'));

        $accessTokenIds = $accessTokens
            ->map(static fn (Token $token): string => (string) $token->getKey())
            ->values()
            ->all();

        if ($accessTokenIds === []) {
            return;
        }

        RefreshToken::query()
            ->whereIn('access_token_id', $accessTokenIds)
            ->where('revoked', false)
            ->update(['revoked' => true]);

        Token::query()
            ->whereIn('id', $accessTokenIds)
            ->where('revoked', false)
            ->update(['revoked' => true]);
    }

    private function sourceGeneration(): int
    {
        $request = $this->currentRequest();
        $grantType = $request->input('grant_type');

        if ($grantType === 'authorization_code') {
            $payload = $this->decryptPayload($this->requiredStringInput($request, 'code'));
            $authCodeId = $payload['auth_code_id'] ?? null;

            if (! is_string($authCodeId) || $authCodeId === '') {
                throw new LogicException('Native OAuth authorization code payload is missing its identifier.');
            }

            $authCode = AuthCode::query()->find($authCodeId);

            if (! $authCode instanceof AuthCode || ! $this->authCodeHasGameTicketScope($authCode)) {
                throw new LogicException('Native OAuth authorization code security context is unavailable.');
            }

            return $this->storedGeneration($authCode->getAttribute('game_auth_generation'));
        }

        if ($grantType === 'refresh_token') {
            $payload = $this->decryptPayload($this->requiredStringInput($request, 'refresh_token'));
            $accessTokenId = $payload['access_token_id'] ?? null;

            if (! is_string($accessTokenId) || $accessTokenId === '') {
                throw new LogicException('Native OAuth refresh payload is missing its access-token identifier.');
            }

            $sourceToken = Token::query()->find($accessTokenId);

            if (! $sourceToken instanceof Token || ! $sourceToken->can('game:ticket')) {
                throw new LogicException('Native OAuth refresh security context is unavailable.');
            }

            return $this->storedGeneration($sourceToken->getAttribute('game_auth_generation'));
        }

        throw new LogicException('The game:ticket scope is restricted to authorization-code and refresh grants.');
    }

    /**
     * @return array<string, mixed>
     */
    private function decryptPayload(string $encrypted): array
    {
        try {
            $json = Crypto::decryptWithPassword(
                $encrypted,
                Passport::tokenEncryptionKey($this->encrypter),
            );
            $payload = json_decode($json, true, 512, JSON_THROW_ON_ERROR);
        } catch (Throwable $exception) {
            throw new LogicException('Native OAuth security context could not be decrypted.', 0, $exception);
        }

        if (! is_array($payload)) {
            throw new LogicException('Native OAuth security context is malformed.');
        }

        return $payload;
    }

    private function currentRequest(): Request
    {
        $request = $this->container->make('request');

        if (! $request instanceof Request) {
            throw new LogicException('Native OAuth request context is unavailable.');
        }

        return $request;
    }

    private function requiredStringInput(Request $request, string $key): string
    {
        $value = $request->input($key);

        if (! is_string($value) || $value === '') {
            throw new LogicException("Native OAuth request is missing {$key}.");
        }

        return $value;
    }

    private function authCodeHasGameTicketScope(AuthCode $authCode): bool
    {
        $rawScopes = $authCode->getAttribute('scopes');

        if (! is_string($rawScopes)) {
            return false;
        }

        try {
            $scopes = json_decode($rawScopes, true, 512, JSON_THROW_ON_ERROR);
        } catch (Throwable) {
            return false;
        }

        return is_array($scopes) && in_array('game:ticket', $scopes, true);
    }

    private function identityGeneration(Identity $identity): int
    {
        return $this->storedGeneration($identity->getAttribute('game_auth_generation'));
    }

    private function storedGeneration(mixed $generation): int
    {
        if (is_int($generation) && $generation >= 0) {
            return $generation;
        }

        if (is_string($generation) && ctype_digit($generation)) {
            return (int) $generation;
        }

        throw new LogicException('Native OAuth game authorization generation is unavailable.');
    }
}
