<?php

namespace App\GameAuth\NativeAdmission;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use InvalidArgumentException;
use JsonException;
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

    private const MIN_TTL_SECONDS = 5;

    private const MAX_TTL_SECONDS = 30;

    /** §8.1 kid grammar, strictly anchored so a trailing newline never reaches the header. */
    private const KEY_ID = '/\A[A-Za-z0-9._-]{1,64}\z/';

    private const JTI = '/\A[A-Za-z0-9_-]{43}\z/';

    /** §8.2 string claims in NativeAdmissionGrantContext constructor order. */
    private const CONTEXT_CLAIMS = [
        'attempt_ref',
        'account_id',
        'character_id',
        'world_id',
        'channel_id',
        'account_security_generation',
        'route_revision',
        'runtime_observation_revision',
        'scope_ownership_generation',
        'ruleset_revision',
        'content_revision',
        'map_revision',
        'world_policy_revision',
        'offer_revision',
    ];

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
        $payload = self::payload($context, $issuedAt, $expiresAt, self::base64Url(random_bytes(32)));
        $signingInput = self::base64Url(self::header($keyId)).'.'.self::base64Url($payload);

        return new IssuedNativeAdmissionGrant(
            $keyId,
            $signingInput,
            $this->signChecked($keyId, $signingInput),
            $issuedAt,
            $expiresAt,
        );
    }

    /**
     * Re-sign a stored signing input with its stored key id; Ed25519 makes the token byte-identical.
     * The stored payload must still be an unexpired grant of this issuer in its exact canonical form
     * (contract §6.2, §8.2), so a tampered attempt row cannot turn the issuer into a signing oracle.
     */
    public function resign(string $keyId, string $signingInput): string
    {
        $prefix = self::base64Url(self::header($keyId)).'.';
        if (strlen($signingInput) > self::MAX_TOKEN_BYTES
            || ! str_starts_with($signingInput, $prefix)
            || preg_match('/\A[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\z/', $signingInput) !== 1) {
            throw new NativeAdmissionUnavailable('Stored native admission signing input does not match its key id.');
        }
        self::assertStoredPayload(substr($signingInput, strlen($prefix)));
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
            ['options' => ['min_range' => self::MIN_TTL_SECONDS, 'max_range' => self::MAX_TTL_SECONDS]],
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
        if (preg_match(self::KEY_ID, $keyId) !== 1) {
            throw new NativeAdmissionUnavailable('Native admission key id is invalid.');
        }
        $header = '{"alg":"Ed25519","kid":"'.$keyId.'","typ":"'.self::TYPE.'"}';
        if (strlen($header) > self::MAX_HEADER_BYTES) {
            throw new NativeAdmissionUnavailable('Native admission grant header exceeds its bound.');
        }

        return $header;
    }

    /** Exactly the §8.2 claims in contract order, without whitespace. */
    private static function payload(NativeAdmissionGrantContext $context, int $issuedAt, int $expiresAt, string $jti): string
    {
        $payload = json_encode([
            'iss' => NativeEvidenceContract::FRESH_ISSUER,
            'aud' => self::AUDIENCE,
            'iat' => $issuedAt,
            'nbf' => $issuedAt,
            'exp' => $expiresAt,
            'jti' => $jti,
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
        if (strlen($payload) > self::MAX_PAYLOAD_BYTES) {
            throw new NativeAdmissionUnavailable('Native admission grant payload exceeds its bound.');
        }

        return $payload;
    }

    /**
     * A stored payload is re-signable only if it decodes within the payload bound, re-validates every
     * claim, is byte-identical to the payload this issuer would build from those claims (so the claim
     * set, order, constants and nbf = iat all hold), has a TTL within 5..30 s and is not expired.
     */
    private static function assertStoredPayload(string $segment): void
    {
        $refuse = static fn (): NativeAdmissionUnavailable => new NativeAdmissionUnavailable(
            'Stored native admission signing input is not a re-signable grant.',
        );

        $json = base64_decode(strtr($segment, '-_', '+/'), true);
        if (! is_string($json) || $json === '' || strlen($json) > self::MAX_PAYLOAD_BYTES
            || self::base64Url($json) !== $segment) {
            throw $refuse();
        }
        try {
            $claims = json_decode($json, true, 2, JSON_THROW_ON_ERROR);
        } catch (JsonException) {
            throw $refuse();
        }
        if (! is_array($claims)) {
            throw $refuse();
        }

        $iat = $claims['iat'] ?? null;
        $exp = $claims['exp'] ?? null;
        $jti = $claims['jti'] ?? null;
        if (! is_int($iat) || ! is_int($exp) || ! is_string($jti) || preg_match(self::JTI, $jti) !== 1
            || $exp - $iat < self::MIN_TTL_SECONDS || $exp - $iat > self::MAX_TTL_SECONDS
            || $exp <= now()->getTimestamp()) {
            throw $refuse();
        }

        $strings = [];
        foreach (self::CONTEXT_CLAIMS as $claim) {
            $value = $claims[$claim] ?? null;
            if (! is_string($value)) {
                throw $refuse();
            }
            $strings[] = $value;
        }
        try {
            $context = new NativeAdmissionGrantContext(...$strings);
            $canonical = self::payload($context, $iat, $exp, $jti);
        } catch (InvalidArgumentException|JsonException) {
            throw $refuse();
        }
        if (! hash_equals($canonical, $json)) {
            throw $refuse();
        }
    }

    private static function base64Url(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }
}
