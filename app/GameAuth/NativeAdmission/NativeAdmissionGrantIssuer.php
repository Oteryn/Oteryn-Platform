<?php

namespace App\GameAuth\NativeAdmission;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use InvalidArgumentException;
use Throwable;

/**
 * FND-04 fresh-entry grant issuer, JWS profile oteryn-pre-admission-v1
 * (OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT §8, §9). It assembles the exact header and claims,
 * refuses to sign unless the key id is trusted in NativeSigningTrustRegistry with the same
 * public key (§9.2 self-check), and re-signs stored signing inputs byte-identically (§6.2).
 */
final class NativeAdmissionGrantIssuer
{
    public const AUDIENCE = 'urn:oteryn:game:admission';

    public const PURPOSE = 'fresh_entry';

    public const TYPE = 'oteryn-admission+jwt';

    public const MAX_TOKEN_BYTES = 4096;

    public const MAX_HEADER_BYTES = 512;

    public const MAX_PAYLOAD_BYTES = 3072;

    private const SELF_CHECK_CACHE_MILLISECONDS = 5000;

    /** @var array<string, int> kid => self-check expiry in Unix milliseconds */
    private array $selfChecked = [];

    public function __construct(
        private readonly NativeAdmissionKeyring $keys,
        private readonly NativeSigningTrustRegistry $trust,
    ) {}

    public function issue(NativeAdmissionGrantContext $context): IssuedNativeAdmissionGrant
    {
        $keyId = $this->keys->currentKeyId();
        $this->assertSignable($keyId);

        $issuedAt = now()->getTimestamp();
        $expiresAt = $issuedAt + $this->ttlSeconds();
        $payload = json_encode([
            'iss' => NativeEvidenceContract::FRESH_ISSUER,
            'aud' => self::AUDIENCE,
            'iat' => $issuedAt,
            'nbf' => $issuedAt,
            'exp' => $expiresAt,
            'jti' => self::base64Url(random_bytes(32)),
            'profile' => NativeEvidenceContract::FRESH_PROFILE,
            'purpose' => self::PURPOSE,
            'attempt_ref' => $context->attemptRef,
            'account_id' => $context->accountId,
            'character_id' => $context->characterId,
            'world_id' => $context->worldId,
            'channel_id' => $context->channelId,
            'account_security_generation' => $context->accountSecurityGeneration,
            'route_revision' => $context->routeRevision,
            'runtime_observation_revision' => $context->runtimeObservationRevision,
            'scope_ownership_generation' => $context->scopeOwnershipGeneration,
            'protocol_major' => 1,
            'transport_profile' => 1,
            'ruleset_revision' => $context->rulesetRevision,
            'content_revision' => $context->contentRevision,
            'map_revision' => $context->mapRevision,
            'world_policy_revision' => $context->worldPolicyRevision,
            'offer_revision' => $context->offerRevision,
        ], JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
        $header = self::header($keyId);
        if (strlen($payload) > self::MAX_PAYLOAD_BYTES) {
            throw new NativeAdmissionUnavailable('Native admission grant payload exceeds its bound.');
        }

        $signingInput = self::base64Url($header).'.'.self::base64Url($payload);

        return new IssuedNativeAdmissionGrant(
            $keyId,
            $signingInput,
            $this->signChecked($keyId, $signingInput),
            $issuedAt,
            $expiresAt,
        );
    }

    /** Re-sign a stored signing input with its stored key id; Ed25519 makes the token byte-identical. */
    public function resign(string $keyId, string $signingInput): string
    {
        $prefix = self::base64Url(self::header($keyId)).'.';
        if (! str_starts_with($signingInput, $prefix)
            || preg_match('/^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/', $signingInput) !== 1) {
            throw new NativeAdmissionUnavailable('Stored native admission signing input does not match its key id.');
        }
        $this->assertSignable($keyId);

        return $this->signChecked($keyId, $signingInput);
    }

    /**
     * Issuer self-check (§9.2): switch on, disposable topology only, key loaded, kid currently
     * trusted in the fixed fresh scope and bound to the public key of the loaded private key.
     */
    public function assertSignable(string $keyId): void
    {
        if (filter_var(config('game-auth.native_admission.enabled'), FILTER_VALIDATE_BOOL) !== true) {
            throw new NativeAdmissionUnavailable('Native admission issuance is disabled.');
        }
        if (! app()->environment(['testing', 'preproduction'])) {
            throw new NativeAdmissionUnavailable('Native admission issuance is restricted to testing or preproduction.');
        }

        $nowMs = now()->getTimestampMs();
        if (($this->selfChecked[$keyId] ?? 0) > $nowMs) {
            return;
        }
        unset($this->selfChecked[$keyId]);

        $keyPurpose = config('game-auth.native_evidence.fresh_key_purpose');
        if (! is_string($keyPurpose) || $keyPurpose === '' || ! $this->keys->has($keyId)) {
            throw new NativeAdmissionUnavailable('Native admission signing key is not configured.');
        }
        try {
            $trusted = $this->trust->trustedPublicKey(
                NativeEvidenceContract::FRESH_ISSUER,
                NativeEvidenceContract::FRESH_PROFILE,
                $keyPurpose,
                $keyId,
            );
        } catch (Throwable) {
            throw new NativeAdmissionUnavailable('Native signing trust could not be read.');
        }
        $loaded = NativeEvidenceContract::encodePublicKey($this->keys->publicKey($keyId));
        if ($trusted === null || ! hash_equals($trusted, $loaded)) {
            throw new NativeAdmissionUnavailable('Native admission key id is not trusted for the loaded key.');
        }

        $this->selfChecked[$keyId] = $nowMs + self::SELF_CHECK_CACHE_MILLISECONDS;
    }

    private function signChecked(string $keyId, string $signingInput): string
    {
        $token = $signingInput.'.'.self::base64Url($this->keys->sign($keyId, $signingInput));
        if (strlen($token) > self::MAX_TOKEN_BYTES) {
            throw new NativeAdmissionUnavailable('Native admission grant exceeds its size bound.');
        }

        return $token;
    }

    private function ttlSeconds(): int
    {
        $ttl = filter_var(
            config('game-auth.native_admission.grant_ttl_seconds'),
            FILTER_VALIDATE_INT,
            ['options' => ['min_range' => 5, 'max_range' => 30]],
        );
        if (! is_int($ttl)) {
            throw new NativeAdmissionUnavailable('Native admission grant TTL must be 5..30 seconds.');
        }

        return $ttl;
    }

    private static function header(string $keyId): string
    {
        try {
            NativeEvidenceContract::assertKeyId($keyId);
        } catch (InvalidArgumentException) {
            throw new NativeAdmissionUnavailable('Native admission key id is invalid.');
        }
        $header = '{"alg":"Ed25519","kid":"'.$keyId.'","typ":"'.self::TYPE.'"}';
        if (strlen($header) > self::MAX_HEADER_BYTES) {
            throw new NativeAdmissionUnavailable('Native admission grant header exceeds its bound.');
        }

        return $header;
    }

    private static function base64Url(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }
}
