<?php

namespace App\Console\Commands;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use App\GameAuth\Worlds\DisposableNativeStore;
use Illuminate\Console\Command;
use LogicException;
use Throwable;

final class PublishNativeTrustedKey extends Command
{
    protected $signature = 'game-auth:native-trust:publish-key
        {--key-id= : Fresh admission signing key id}
        {--public-key-file= : Regular non-symlink file holding the 32 raw Ed25519 public key bytes}';

    protected $description = 'Trust one fresh admission public key in a disposable preproduction run (testing/preproduction only).';

    public function handle(NativeSigningTrustRegistry $registry, DisposableNativeStore $store): int
    {
        $keyId = $this->option('key-id');
        $keyFile = $this->option('public-key-file');
        $keyPurpose = config('game-auth.native_evidence.fresh_key_purpose');
        if (! is_string($keyId) || $keyId === '' || ! is_string($keyFile) || $keyFile === '') {
            $this->components->error('Supply --key-id and --public-key-file.');

            return self::FAILURE;
        }

        try {
            // publishTrustedKey writes the floor file, the witness-store lock file and the rows through the
            // default connection and the high-water directory, so both are fenced to this run before any write.
            [$connection, $runDirectory] = $store->retainedRun();
            $store->assertHighWaterDirectoryWithin($runDirectory);
            if (! is_string($keyPurpose) || $keyPurpose === '') {
                throw new LogicException('The fresh admission key purpose is not configured.');
            }
            NativeEvidenceContract::assertKeyId($keyId);
            $publicKey = $this->publicKeyBytes($keyFile);

            $stored = $registry->publishTrustedKey(
                NativeEvidenceContract::FRESH_ISSUER,
                NativeEvidenceContract::FRESH_PROFILE,
                $keyPurpose,
                $keyId,
                $publicKey,
            );
            $profileVersion = $connection->table('native_game_signing_trust_profiles')
                ->where('id', $stored->profile_id ?? null)->value('profile_version');
            if (($stored->key_id ?? null) !== $keyId || ! is_numeric($profileVersion)) {
                throw new LogicException('The trusted key does not read back.');
            }

            $json = json_encode([
                'key_id' => $keyId,
                'profile_version' => (int) $profileVersion,
            ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
        } catch (Throwable) {
            $this->components->error('Disposable native trusted key publication failed.');

            return self::FAILURE;
        }

        $this->line($json);

        return self::SUCCESS;
    }

    private function publicKeyBytes(string $path): string
    {
        if (is_link($path) || ! is_file($path)) {
            throw new LogicException('The public key file must be a regular file, not a symlink.');
        }

        $bytes = file_get_contents($path, false, null, 0, 33);
        if (! is_string($bytes) || strlen($bytes) !== 32) {
            throw new LogicException('The public key file must hold exactly 32 raw bytes.');
        }

        return $bytes;
    }
}
