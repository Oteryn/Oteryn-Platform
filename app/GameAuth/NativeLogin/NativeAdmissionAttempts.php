<?php

namespace App\GameAuth\NativeLogin;

use App\GameAuth\NativeAdmission\NativeAdmissionGrantContext;
use App\GameAuth\NativeAdmission\NativeAdmissionGrantIssuer;
use App\GameAuth\NativeAdmission\NativeAdmissionUnavailable;
use Illuminate\Database\UniqueConstraintViolationException;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use JsonException;
use Throwable;

/**
 * Native admission issuer transaction with attempt_ref idempotency (contract §6.2):
 * one attempt_ref yields at most one logical grant. A first request redeems the native ticket,
 * resolves Character and route, signs and stores the signing input; a retry re-signs the stored
 * input byte-identically after binding it to its attempt row.
 */
final class NativeAdmissionAttempts
{
    /** Set once the transaction body finished, so a later failure is an unknown commit outcome. */
    private bool $committing = false;

    public function __construct(
        private readonly NativeGameLoginTickets $tickets,
        private readonly NativeAdmissionScopeResolver $scopes,
        private readonly NativeAdmissionGrantIssuer $issuer,
    ) {}

    public function admit(NativeAdmissionRequest $request): NativeAdmissionResult
    {
        if (filter_var(config('game-auth.native_admission.enabled'), FILTER_VALIDATE_BOOL) !== true) {
            throw new NativeLoginRefused(NativeLoginError::Unavailable);
        }

        try {
            return $this->attempt($request);
        } catch (NativeAdmissionAttemptRace|UniqueConstraintViolationException) {
            // A concurrent request for this attempt_ref committed first: answer from its record.
        }
        try {
            return $this->attempt($request);
        } catch (NativeAdmissionAttemptRace|UniqueConstraintViolationException) {
            throw new NativeLoginRefused(NativeLoginError::ReconciliationRequired);
        }
    }

    private function attempt(NativeAdmissionRequest $request): NativeAdmissionResult
    {
        $this->committing = false;
        try {
            return DB::transaction(function () use ($request): NativeAdmissionResult {
                $existing = NativeAdmissionAttempt::query()
                    ->where('attempt_ref', $request->attemptRef)
                    ->lockForUpdate()
                    ->first();
                $result = $existing instanceof NativeAdmissionAttempt
                    ? $this->replay($existing, $request)
                    : $this->issue($request);
                $this->committing = true;

                return $result;
            });
        } catch (NativeLoginRefused|NativeAdmissionAttemptRace|UniqueConstraintViolationException $refused) {
            throw $refused;
        } catch (NativeAdmissionUnavailable) {
            throw new NativeLoginRefused(NativeLoginError::Unavailable);
        } catch (Throwable) {
            // Unknown commit outcome: only the same attempt_ref and ticket can resolve it.
            throw new NativeLoginRefused($this->committing ? NativeLoginError::ReconciliationRequired : NativeLoginError::Unavailable);
        }
    }

    private function issue(NativeAdmissionRequest $request): NativeAdmissionResult
    {
        $account = $this->tickets->redeem($request->ticket(), $request->attemptRef);
        $scope = $this->scopes->resolve($account, $request);
        if ($scope->characterId !== $request->characterId) {
            throw new NativeLoginRefused(NativeLoginError::CharacterConflict);
        }
        if ($request->channelId !== null && $scope->channelId !== $request->channelId) {
            throw new NativeLoginRefused(NativeLoginError::RouteUnavailable);
        }

        try {
            $context = new NativeAdmissionGrantContext(
                attemptRef: $request->attemptRef,
                accountId: $account->accountId,
                characterId: $scope->characterId,
                worldId: $scope->worldId,
                channelId: $scope->channelId,
                accountSecurityGeneration: $account->accountSecurityGeneration,
                routeRevision: $scope->routeRevision,
                runtimeObservationRevision: $scope->runtimeObservationRevision,
                scopeOwnershipGeneration: $scope->scopeOwnershipGeneration,
                rulesetRevision: $scope->rulesetRevision,
                contentRevision: $scope->contentRevision,
                mapRevision: $scope->mapRevision,
                worldPolicyRevision: $scope->worldPolicyRevision,
                offerRevision: $scope->offerRevision,
            );
        } catch (InvalidArgumentException) {
            throw new NativeLoginRefused(NativeLoginError::Unavailable);
        }
        $grant = $this->issuer->issue($context);

        NativeAdmissionAttempt::query()->create([
            'attempt_ref' => $request->attemptRef,
            'ticket_hash' => $this->tickets->hash($request->ticket()),
            'account_id' => $account->accountId,
            'character_id' => $scope->characterId,
            'requested_channel_id' => $request->channelId,
            'world_id' => $scope->worldId,
            'channel_id' => $scope->channelId,
            'offer_digest' => $request->offerDigest,
            'signing_input' => $grant->signingInput,
            'key_id' => $grant->keyId,
            'issued_at' => $grant->issuedAt,
            'expires_at' => $grant->expiresAt,
        ]);

        return $this->result($request, $scope->worldId, $scope->channelId, $scope->routeRevision, $grant->token, $grant->expiresAt);
    }

    private function replay(NativeAdmissionAttempt $attempt, NativeAdmissionRequest $request): NativeAdmissionResult
    {
        if (! hash_equals($attempt->ticket_hash, $this->tickets->hash($request->ticket()))
            || $attempt->character_id !== $request->characterId
            || $attempt->requested_channel_id !== $request->channelId
            || ! hash_equals($attempt->offer_digest, $request->offerDigest)) {
            throw new NativeLoginRefused(NativeLoginError::AttemptConflict);
        }
        $signingInput = $attempt->signing_input;
        if ($attempt->expires_at * 1000 - now()->getTimestampMs() < 1000 || $signingInput === null) {
            throw new NativeLoginRefused(NativeLoginError::GrantExpired);
        }

        $routeRevision = self::boundRouteRevision($attempt, $signingInput);
        try {
            $token = $this->issuer->resign($attempt->key_id, $signingInput);
        } catch (NativeAdmissionUnavailable) {
            throw new NativeLoginRefused(NativeLoginError::ReconciliationRequired);
        }

        return $this->result($request, $attempt->world_id, $attempt->channel_id, $routeRevision, $token, $attempt->expires_at);
    }

    /**
     * Binds the stored payload to its attempt row, so a signing input copied from another row or
     * altered in storage is never re-signed; the issuer re-validates its canonical form (§6.2).
     */
    private static function boundRouteRevision(NativeAdmissionAttempt $attempt, string $signingInput): string
    {
        $segments = explode('.', $signingInput);
        $json = count($segments) === 2 ? base64_decode(strtr($segments[1], '-_', '+/'), true) : false;
        try {
            $claims = is_string($json) ? json_decode($json, true, 2, JSON_THROW_ON_ERROR) : null;
        } catch (JsonException) {
            $claims = null;
        }

        $expected = [
            'attempt_ref' => $attempt->attempt_ref,
            'account_id' => $attempt->account_id,
            'character_id' => $attempt->character_id,
            'world_id' => $attempt->world_id,
            'channel_id' => $attempt->channel_id,
            'iat' => $attempt->issued_at,
            'exp' => $attempt->expires_at,
        ];
        $routeRevision = is_array($claims) ? ($claims['route_revision'] ?? null) : null;
        if (! is_array($claims) || ! is_string($routeRevision)) {
            throw new NativeLoginRefused(NativeLoginError::ReconciliationRequired);
        }
        foreach ($expected as $claim => $value) {
            if (($claims[$claim] ?? null) !== $value) {
                throw new NativeLoginRefused(NativeLoginError::ReconciliationRequired);
            }
        }

        return $routeRevision;
    }

    private function result(
        NativeAdmissionRequest $request,
        string $worldId,
        string $channelId,
        string $routeRevision,
        string $token,
        int $expiresAt,
    ): NativeAdmissionResult {
        $remainingMs = $expiresAt * 1000 - now()->getTimestampMs();
        if ($remainingMs < 1000) {
            throw new NativeLoginRefused(NativeLoginError::GrantExpired);
        }

        return new NativeAdmissionResult($request->attemptRef, $worldId, $channelId, $routeRevision, $token, intdiv($remainingMs, 1000));
    }
}
