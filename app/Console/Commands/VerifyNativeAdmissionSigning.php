<?php

namespace App\Console\Commands;

use App\GameAuth\NativeAdmission\NativeAdmissionGrantContext;
use App\GameAuth\NativeAdmission\NativeAdmissionGrantIssuer;
use App\GameAuth\NativeAdmission\NativeAdmissionKeyring;
use App\GameAuth\NativeAdmission\NativeAdmissionUnavailable;
use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use Illuminate\Console\Command;
use JsonException;
use Throwable;

/**
 * Native admission signing self-check for deployment gates (contract §9.2). It runs the issuer
 * self-check for the configured current key, signs one throwaway fresh-entry grant through the real
 * issuer, verifies that grant against the public key the trust registry holds for its kid (not the
 * key derived from the loaded seed) and re-signs it byte-identically.
 *
 * The grant binds fixed synthetic identifiers, is held in memory only and is never printed, logged
 * or stored. Output carries the key id and a reason, never key, signature or token bytes.
 * Exit 0: the issuer can sign and the grant verifies. Exit 1: it cannot, and one error line names the reason.
 */
final class VerifyNativeAdmissionSigning extends Command
{
    private const SYNTHETIC_REVISION = 'self-check.0';

    protected $signature = 'game-auth:native-admission:self-check';

    protected $description = 'Sign and verify a throwaway native admission grant with the configured key and trust (exit 0 signs, 1 refused; testing/preproduction only).';

    public function handle(NativeAdmissionKeyring $keys, NativeSigningTrustRegistry $trust): int
    {
        try {
            $keyId = $this->check($keys, $trust);
        } catch (NativeAdmissionUnavailable $exception) {
            // Issuer and keyring messages are internal diagnostics that never carry key material.
            return $this->refuse($exception->getMessage());
        } catch (Throwable $exception) {
            return $this->refuse('Unexpected '.class_basename($exception).' during the signing self-check.');
        }

        $this->line((string) json_encode(['result' => 'ok', 'key_id' => $keyId], JSON_UNESCAPED_SLASHES));

        return self::SUCCESS;
    }

    private function check(NativeAdmissionKeyring $keys, NativeSigningTrustRegistry $trust): string
    {
        $issuer = new NativeAdmissionGrantIssuer($keys, $trust);

        $keyId = $keys->currentKeyId();
        $issuer->assertSignable($keyId);

        $grant = $issuer->issue(self::syntheticContext());
        if ($grant->keyId !== $keyId) {
            throw new NativeAdmissionUnavailable('Self-check grant was signed with an unexpected key id.');
        }

        $segments = explode('.', $grant->token);
        if (count($segments) !== 3 || $grant->signingInput !== $segments[0].'.'.$segments[1]) {
            throw new NativeAdmissionUnavailable('Self-check grant is not a compact JWS over its signing input.');
        }
        $header = self::decode($segments[0]);
        if ($header !== '{"alg":"Ed25519","kid":"'.$keyId.'","typ":"'.NativeAdmissionGrantIssuer::TYPE.'"}') {
            throw new NativeAdmissionUnavailable('Self-check grant header does not match the issuer profile.');
        }
        $this->assertClaims(self::decode($segments[1]));

        $this->assertVerifiesWithTrustedKey($trust, $keyId, $grant->signingInput, self::decode($segments[2]));

        if (! hash_equals($grant->token, $issuer->resign($keyId, $grant->signingInput))) {
            throw new NativeAdmissionUnavailable('Self-check grant does not re-sign byte-identically.');
        }

        return $keyId;
    }

    /** Verifies with the registry's public key for the kid, so a key-to-trust mismatch fails here too. */
    private function assertVerifiesWithTrustedKey(
        NativeSigningTrustRegistry $trust,
        string $keyId,
        string $signingInput,
        string $signature,
    ): void {
        $keyPurpose = config('game-auth.native_evidence.fresh_key_purpose');
        $trusted = is_string($keyPurpose) && $keyPurpose !== ''
            ? $trust->trustedPublicKey(
                NativeEvidenceContract::FRESH_ISSUER,
                NativeEvidenceContract::FRESH_PROFILE,
                $keyPurpose,
                $keyId,
            )
            : null;
        if ($trusted === null) {
            throw new NativeAdmissionUnavailable('Native admission key id is not trusted for the loaded key.');
        }
        $publicKey = self::decode($trusted);
        if (strlen($publicKey) !== SODIUM_CRYPTO_SIGN_PUBLICKEYBYTES || strlen($signature) !== SODIUM_CRYPTO_SIGN_BYTES) {
            throw new NativeAdmissionUnavailable('Self-check grant signature or trusted public key has the wrong length.');
        }
        if (! sodium_crypto_sign_verify_detached($signature, $signingInput, $publicKey)) {
            throw new NativeAdmissionUnavailable('Self-check grant does not verify with the trusted public key.');
        }
        // A verifier that accepts everything would make the check above meaningless.
        if (sodium_crypto_sign_verify_detached($signature, $signingInput.'.', $publicKey)) {
            throw new NativeAdmissionUnavailable('Self-check verifier accepted a modified signing input.');
        }
    }

    private function assertClaims(string $payload): void
    {
        try {
            $claims = json_decode($payload, true, 2, JSON_THROW_ON_ERROR);
        } catch (JsonException) {
            throw new NativeAdmissionUnavailable('Self-check grant payload is not JSON.');
        }
        $iat = is_array($claims) ? ($claims['iat'] ?? null) : null;
        $exp = is_array($claims) ? ($claims['exp'] ?? null) : null;
        if (! is_array($claims)
            || ($claims['iss'] ?? null) !== NativeEvidenceContract::FRESH_ISSUER
            || ($claims['aud'] ?? null) !== NativeAdmissionGrantIssuer::AUDIENCE
            || ($claims['profile'] ?? null) !== NativeEvidenceContract::FRESH_PROFILE
            || ($claims['purpose'] ?? null) !== NativeAdmissionGrantIssuer::PURPOSE
            || ($claims['attempt_ref'] ?? null) !== self::syntheticId(1)
            || ! is_int($iat) || ! is_int($exp) || ($claims['nbf'] ?? null) !== $iat
            || $exp <= now()->getTimestamp()) {
            throw new NativeAdmissionUnavailable('Self-check grant claims do not match the issuer profile.');
        }
    }

    private function refuse(string $reason): int
    {
        $this->components->error('Native admission signing self-check failed: '.$reason);

        return self::FAILURE;
    }

    private static function syntheticContext(): NativeAdmissionGrantContext
    {
        return new NativeAdmissionGrantContext(
            attemptRef: self::syntheticId(1),
            accountId: self::syntheticId(2),
            characterId: self::syntheticId(3),
            worldId: self::syntheticId(4),
            channelId: self::syntheticId(5),
            accountSecurityGeneration: '1',
            routeRevision: self::SYNTHETIC_REVISION,
            runtimeObservationRevision: self::SYNTHETIC_REVISION,
            scopeOwnershipGeneration: '1',
            rulesetRevision: self::SYNTHETIC_REVISION,
            contentRevision: self::SYNTHETIC_REVISION,
            mapRevision: self::SYNTHETIC_REVISION,
            worldPolicyRevision: self::SYNTHETIC_REVISION,
            offerRevision: self::SYNTHETIC_REVISION,
        );
    }

    /** Fixed all-zero UUIDv7 shape; it names no real attempt, account, character, world or channel. */
    private static function syntheticId(int $n): string
    {
        return sprintf('00000000-0000-7000-8000-%012d', $n);
    }

    private static function decode(string $segment): string
    {
        $decoded = base64_decode(strtr($segment, '-_', '+/'), true);
        if (! is_string($decoded) || rtrim(strtr(base64_encode($decoded), '+/', '-_'), '=') !== $segment) {
            throw new NativeAdmissionUnavailable('Self-check grant is not canonical base64url.');
        }

        return $decoded;
    }
}
