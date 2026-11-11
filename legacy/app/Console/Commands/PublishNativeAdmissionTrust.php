<?php

namespace App\Console\Commands;

use App\GameAuth\NativeAdmission\NativeAdmissionKeyring;
use App\GameAuth\NativeAdmission\NativeAdmissionUnavailable;
use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeEvidencePublicationFence;
use App\GameAuth\NativeEvidence\NativeEvidenceUnavailable;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use Illuminate\Console\Command;
use InvalidArgumentException;
use LogicException;
use Throwable;

/**
 * Publishes the public half of the configured current native admission signing key into the
 * deployment's own signing trust registry (contract §9.2, §9.3 step 2) so that the issuer
 * self-check and Game's ReadFreshSigningTrustV1 can find it. Owner decision D1006: native login on
 * the deployed preproduction stack with owner key custody, where the disposable-store command
 * game-auth:native-trust:publish-key cannot run.
 *
 * Fail closed, nothing written on refusal: testing/preproduction only, native admission switched
 * on, fresh key purpose configured, current key loaded through the issuer keyring (the same
 * ownership and mode checks as signing). The public key is derived from the loaded private key, so
 * only the key the issuer will really sign with can be trusted. All registry rules stay in force:
 * a kid is bound to one public key forever, a revoked kid is never re-trusted, at most two keys
 * are trusted at once, and every write passes the trust-state high-water witness.
 *
 * Idempotent: a kid already trusted for the same public key is reported and no registry row is
 * written, but the high-water witness is still checked on every run.
 * Output is one JSON line with the trust scope, key id and PUBLIC key, or one reason line; never
 * seed, private key or signature bytes.
 */
final class PublishNativeAdmissionTrust extends Command
{
    protected $signature = 'game-auth:native-admission:publish-trust
        {--first-publication-fence= : base64url JSON {run_id, intent_sha256} naming the open attempt intent (PostgreSQL P6a)}';

    protected $description = 'Trust the configured current native admission signing public key in the signing trust registry (testing/preproduction only; prints key id and public key).';

    public function handle(NativeAdmissionKeyring $keys, NativeSigningTrustRegistry $trust): int
    {
        try {
            $published = $this->publish($keys, $trust);
        } catch (NativeAdmissionUnavailable|NativeEvidenceUnavailable|LogicException $exception) {
            // Keyring, witness and registry messages are fixed diagnostics that never carry key material.
            return $this->refuse($exception->getMessage());
        } catch (Throwable $exception) {
            return $this->refuse('Unexpected '.class_basename($exception).' during trust publication.');
        }

        $this->line((string) json_encode($published, JSON_UNESCAPED_SLASHES));

        return self::SUCCESS;
    }

    /** @return array{result: string, issuer: string, profile: string, key_purpose: string, key_id: string, public_key: string, fence?: array{run_id: string, attempt: int, intent_sha256: string}|null} */
    private function publish(NativeAdmissionKeyring $keys, NativeSigningTrustRegistry $trust): array
    {
        if (! app()->environment(['testing', 'preproduction'])) {
            throw new NativeAdmissionUnavailable('Native admission trust publication is restricted to testing or preproduction.');
        }
        if (filter_var(config('game-auth.native_admission.enabled'), FILTER_VALIDATE_BOOL) !== true) {
            throw new NativeAdmissionUnavailable('Native admission is disabled; no signing key is trusted.');
        }
        $keyPurpose = config('game-auth.native_evidence.fresh_key_purpose');
        if (! is_string($keyPurpose) || $keyPurpose === '') {
            throw new NativeAdmissionUnavailable('The fresh admission key purpose is not configured.');
        }

        $fenceOption = $this->option('first-publication-fence');
        $fence = is_string($fenceOption) && $fenceOption !== '' ? NativeEvidencePublicationFence::fromEncoded($fenceOption) : null;

        $keyId = $keys->currentKeyId();
        if (preg_match('/\A[A-Za-z0-9._-]{1,64}\z/', $keyId) !== 1 || ! $keys->has($keyId)) {
            throw new NativeAdmissionUnavailable('Native admission current signing key is not loaded.');
        }
        $publicKeyBytes = $keys->publicKey($keyId);
        $publicKey = NativeEvidenceContract::encodePublicKey($publicKeyBytes);

        $before = $trust->trustedPublicKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            $keyPurpose,
            $keyId,
        );
        // Always take the witnessed write path, even for a kid that already reads as trusted: only
        // publishTrustedKey checks the trust-state high-water, so a lost, replaced or rolled-back
        // witness (or a database restored below it) fails closed here instead of reporting trust.
        // For the same kid and public key it writes no registry row.
        $stored = $trust->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            $keyPurpose,
            $keyId,
            $publicKeyBytes,
            $fence,
        );
        if (($stored->key_id ?? null) !== $keyId) {
            throw new LogicException('The trusted key does not read back.');
        }
        $trusted = $trust->trustedPublicKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            $keyPurpose,
            $keyId,
        );
        $result = $before === null ? 'published' : 'already_trusted';
        if ($trusted === null || ! hash_equals($trusted, $publicKey)) {
            throw new InvalidArgumentException('Native admission key id is trusted for a different public key; a kid is bound to one public key forever.');
        }

        $published = [
            'result' => $result,
            'issuer' => NativeEvidenceContract::FRESH_ISSUER,
            'profile' => NativeEvidenceContract::FRESH_PROFILE,
            'key_purpose' => $keyPurpose,
            'key_id' => $keyId,
            'public_key' => $publicKey,
        ];
        if ($fence instanceof NativeEvidencePublicationFence) {
            // Null when the fence was not needed (a profile already existed or the setting is off).
            $published['fence'] = $fence->result();
        }

        return $published;
    }

    private function refuse(string $reason): int
    {
        $this->components->error('Native admission trust publication failed: '.$reason);

        return self::FAILURE;
    }
}
