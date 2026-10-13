<?php

namespace Tests\Feature\GameAuth\NativeAdmission;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use Closure;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Log\Events\MessageLogged;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Event;
use PHPUnit\Framework\Attributes\DataProvider;
use stdClass;
use Tests\TestCase;

final class NativeAdmissionPublishTrustCommandTest extends TestCase
{
    use RefreshDatabase;

    private const COMMAND = 'game-auth:native-admission:publish-trust';

    private const SELF_CHECK = 'game-auth:native-admission:self-check';

    private const PURPOSE = 'fresh_admission';

    private string $directory;

    protected function setUp(): void
    {
        parent::setUp();

        $this->directory = storage_path('framework/testing/native-admission-publish-trust-'.bin2hex(random_bytes(6)));
        mkdir($this->directory.'/witness', 0700, true);
        config([
            'game-auth.native_evidence.high_water_directory' => $this->directory.'/witness',
            'game-auth.native_evidence.fresh_key_purpose' => self::PURPOSE,
            'game-auth.native_admission.enabled' => true,
            'game-auth.native_admission.grant_ttl_seconds' => 20,
            'game-auth.native_admission.signing_key_file' => null,
            'game-auth.native_admission.signing_key_id' => null,
            'game-auth.native_admission.retiring_signing_key_file' => null,
            'game-auth.native_admission.retiring_signing_key_id' => null,
        ]);
    }

    protected function tearDown(): void
    {
        foreach (glob($this->directory.'/{,witness/}*', GLOB_BRACE) ?: [] as $path) {
            (is_link($path) || is_file($path)) && @unlink($path);
        }
        @rmdir($this->directory.'/witness');
        @rmdir($this->directory);

        parent::tearDown();
    }

    public function test_malformed_first_publication_fence_refuses_and_writes_nothing(): void
    {
        $this->installKey('admission-1');
        $before = $this->registryState();

        foreach (['!!not-base64!!', $this->encode('not json'), $this->encode('{"run_id":"x","intent_sha256":"y"}')] as $value) {
            self::assertSame(1, Artisan::call(self::COMMAND, ['--first-publication-fence' => $value]));
            self::assertSame($before, $this->registryState());
            self::assertSame([], glob($this->directory.'/witness/*') ?: []);
        }
    }

    public function test_valid_first_publication_fence_is_ignored_while_the_setting_is_off(): void
    {
        config(['game-auth.native_evidence.first_publication_fence' => 'off']);
        $this->installKey('admission-1');
        $fence = $this->encode((string) json_encode([
            'run_id' => '11111111-1111-4111-8111-111111111111',
            'intent_sha256' => str_repeat('a', 64),
        ]));

        self::assertSame(0, Artisan::call(self::COMMAND, ['--first-publication-fence' => $fence]));
        $result = json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR);

        self::assertSame('published', $result['result']);
        self::assertArrayHasKey('fence', $result);
        self::assertNull($result['fence']);
    }

    public function test_result_has_no_fence_field_without_the_option(): void
    {
        $this->installKey('admission-1');

        self::assertSame(0, Artisan::call(self::COMMAND));
        self::assertArrayNotHasKey('fence', json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR));
    }

    public function test_publishes_the_loaded_current_public_key_and_the_self_check_then_passes(): void
    {
        Event::fake([MessageLogged::class]);
        $seed = $this->installKey('admission-1');

        self::assertSame(1, $this->exitCode(self::SELF_CHECK), 'An untrusted key must not sign before publication.');

        self::assertSame(0, $this->exitCode(self::COMMAND));
        $output = Artisan::output();

        self::assertSame([
            'result' => 'published',
            'issuer' => NativeEvidenceContract::FRESH_ISSUER,
            'profile' => NativeEvidenceContract::FRESH_PROFILE,
            'key_purpose' => self::PURPOSE,
            'key_id' => 'admission-1',
            'public_key' => $this->encode($this->publicKey($seed)),
        ], json_decode($output, true, 512, JSON_THROW_ON_ERROR));
        $this->assertNoSecretMaterial($seed, $output);
        Event::assertNotDispatched(MessageLogged::class, $this->mentions($seed));
        self::assertSame($this->encode($this->publicKey($seed)), $this->trustedPublicKey('admission-1'));

        self::assertSame(0, $this->exitCode(self::SELF_CHECK));
    }

    public function test_second_run_reports_already_trusted_and_writes_nothing(): void
    {
        $seed = $this->installKey('admission-1');
        self::assertSame(0, $this->exitCode(self::COMMAND));
        $before = $this->registryState();

        self::assertSame(0, $this->exitCode(self::COMMAND));
        $decoded = json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR);

        self::assertIsArray($decoded);
        self::assertSame('already_trusted', $decoded['result'] ?? null);
        self::assertSame($this->encode($this->publicKey($seed)), $decoded['public_key'] ?? null);
        self::assertSame($before, $this->registryState());
    }

    public function test_rotation_trusts_the_new_current_key_next_to_the_retiring_one(): void
    {
        $retiring = $this->installKey('admission-1');
        self::assertSame(0, $this->exitCode(self::COMMAND));

        $this->installKey('admission-1', 'retiring', $retiring);
        $current = $this->installKey('admission-2');
        self::assertSame(0, $this->exitCode(self::COMMAND));

        self::assertSame($this->encode($this->publicKey($retiring)), $this->trustedPublicKey('admission-1'));
        self::assertSame($this->encode($this->publicKey($current)), $this->trustedPublicKey('admission-2'));
        self::assertSame(0, $this->exitCode(self::SELF_CHECK));
    }

    public function test_already_trusted_key_fails_closed_when_the_witness_was_lost(): void
    {
        $seed = $this->installKey('admission-1');
        self::assertSame(0, $this->exitCode(self::COMMAND));
        foreach (glob($this->directory.'/witness/*') ?: [] as $path) {
            unlink($path);
        }
        $before = $this->registryState();

        self::assertSame(1, $this->exitCode(self::COMMAND));
        $output = Artisan::output();

        self::assertStringContainsString('Native admission trust publication failed:', $output);
        self::assertStringNotContainsString('already_trusted', $output);
        $this->assertNoSecretMaterial($seed, $output);
        self::assertSame($before, $this->registryState());
    }

    public function test_already_trusted_key_fails_closed_when_the_database_was_restored_below_the_witness(): void
    {
        $retiring = $this->installKey('admission-1');
        self::assertSame(0, $this->exitCode(self::COMMAND));
        $restored = $this->registryState();
        $this->installKey('admission-1', 'retiring', $retiring);
        $this->installKey('admission-2');
        self::assertSame(0, $this->exitCode(self::COMMAND));

        // Simulate restoring the pre-rotation database while the witness keeps its high-water.
        DB::table('native_game_signing_trust_key_versions')->where('key_id', 'admission-2')->delete();
        foreach ($restored['profiles'] as $profile) {
            DB::table('native_game_signing_trust_profiles')->where('id', $profile['id'])->update([
                'profile_version' => $profile['profile_version'],
                'issuer_revision' => $profile['issuer_revision'],
                'revoked_at' => $profile['revoked_at'],
                'updated_at' => $profile['updated_at'],
            ]);
        }
        self::assertSame($restored, $this->registryState());
        config([
            'game-auth.native_admission.retiring_signing_key_file' => null,
            'game-auth.native_admission.retiring_signing_key_id' => null,
        ]);
        $this->installKey('admission-1', 'current', $retiring);

        self::assertSame(1, $this->exitCode(self::COMMAND));
        $output = Artisan::output();

        self::assertStringContainsString('does not match retained high-water', $output);
        self::assertStringNotContainsString('already_trusted', $output);
        self::assertSame($restored, $this->registryState());
    }

    #[DataProvider('refusals')]
    public function test_refusal_exits_one_with_a_reason_writes_nothing_and_prints_no_secret(string $case, string $reason): void
    {
        Event::fake([MessageLogged::class]);
        $seed = $this->installKey('admission-1');
        $this->arrange($case, $seed);
        $before = $this->registryState();

        self::assertSame(1, $this->exitCode(self::COMMAND));
        $output = Artisan::output();

        self::assertStringContainsString('Native admission trust publication failed:', $output);
        self::assertStringContainsString($reason, $output);
        $this->assertNoSecretMaterial($seed, $output);
        Event::assertNotDispatched(MessageLogged::class, $this->mentions($seed));
        self::assertSame($before, $this->registryState());
    }

    /** @return array<string, array{string, string}> */
    public static function refusals(): array
    {
        return [
            'production environment' => ['production', 'restricted to testing or preproduction'],
            'staging environment' => ['staging', 'restricted to testing or preproduction'],
            'native admission off' => ['switch off', 'Native admission is disabled'],
            'key purpose not configured' => ['no key purpose', 'fresh admission key purpose is not configured'],
            'key file not configured' => ['no key file', 'current key file must be an absolute path'],
            'key id not configured' => ['no key id', 'current key id is not configured'],
            'key file readable by group' => ['group readable', 'must not be accessible to group or others'],
            'key file holds no seed' => ['no seed', 'canonical base64url 32-byte Ed25519 seed'],
            'kid trusted for another public key' => ['other public key', 'bound to one public key forever'],
            'kid revoked' => ['revoked', 'cannot be re-trusted'],
            'two keys already trusted' => ['two trusted', 'At most two fresh admission signing keys'],
            'witness not configured' => ['no witness', 'high-water witness is not configured'],
            'witness missing' => ['missing witness', 'high-water witness is unavailable'],
        ];
    }

    /** @param non-empty-string $seed */
    private function arrange(string $case, string $seed): void
    {
        switch ($case) {
            case 'production':
            case 'staging':
                $this->app['env'] = $case;
                break;
            case 'switch off':
                config(['game-auth.native_admission.enabled' => false]);
                break;
            case 'no key purpose':
                config(['game-auth.native_evidence.fresh_key_purpose' => '']);
                break;
            case 'no key file':
                config(['game-auth.native_admission.signing_key_file' => null]);
                break;
            case 'no key id':
                config(['game-auth.native_admission.signing_key_id' => null]);
                break;
            case 'group readable':
                chmod($this->directory.'/current.key', 0640);
                break;
            case 'no seed':
                file_put_contents($this->directory.'/current.key', "not-a-seed\n");
                break;
            case 'other public key':
                $this->trust('admission-1', str_repeat("\x05", 32));
                break;
            case 'revoked':
                $this->trust('admission-1', $this->publicKey($seed));
                $this->registry()->revokeKey(
                    NativeEvidenceContract::FRESH_ISSUER,
                    NativeEvidenceContract::FRESH_PROFILE,
                    self::PURPOSE,
                    'admission-1',
                );
                break;
            case 'two trusted':
                $this->trust('admission-old-1', str_repeat("\x06", 32));
                $this->trust('admission-old-2', str_repeat("\x07", 32));
                break;
            case 'no witness':
                config(['game-auth.native_evidence.high_water_directory' => null]);
                break;
            case 'missing witness':
                config(['game-auth.native_evidence.high_water_directory' => $this->directory.'/absent']);
                break;
        }
    }

    /** @return array{profiles: list<array<mixed>>, keys: list<array<mixed>>} */
    private function registryState(): array
    {
        return [
            'profiles' => $this->rows('native_game_signing_trust_profiles'),
            'keys' => $this->rows('native_game_signing_trust_key_versions'),
        ];
    }

    /** @return list<array<mixed>> */
    private function rows(string $table): array
    {
        $rows = [];
        foreach (DB::table($table)->orderBy('id')->get() as $row) {
            self::assertInstanceOf(stdClass::class, $row);
            $rows[] = get_object_vars($row);
        }

        return $rows;
    }

    private function trustedPublicKey(string $keyId): ?string
    {
        return $this->registry()->trustedPublicKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            self::PURPOSE,
            $keyId,
        );
    }

    /** @param non-empty-string $seed */
    private function assertNoSecretMaterial(string $seed, string $output): void
    {
        $secretKey = sodium_crypto_sign_secretkey(sodium_crypto_sign_seed_keypair($seed));
        foreach ([$seed, $this->encode($seed), $secretKey, $this->encode($secretKey), bin2hex($seed), bin2hex($secretKey)] as $material) {
            self::assertStringNotContainsString($material, $output);
        }
    }

    /**
     * @param  non-empty-string  $seed
     * @return Closure(MessageLogged): bool
     */
    private function mentions(string $seed): Closure
    {
        $needle = $this->encode($seed);

        return static fn (MessageLogged $event): bool => str_contains($event->message.print_r($event->context, true), $needle);
    }

    /**
     * Writes a test-only generated seed (or the given one) and points the given key role at it.
     *
     * @param  non-empty-string|null  $seed
     * @return non-empty-string
     */
    private function installKey(string $keyId, string $role = 'current', ?string $seed = null): string
    {
        $seed ??= random_bytes(SODIUM_CRYPTO_SIGN_SEEDBYTES);
        $path = $this->directory.'/'.$role.'.key';
        file_put_contents($path, $this->encode($seed)."\n");
        chmod($path, 0600);
        $prefix = $role === 'current' ? 'game-auth.native_admission.' : 'game-auth.native_admission.retiring_';
        config([$prefix.'signing_key_file' => $path, $prefix.'signing_key_id' => $keyId]);

        return $seed;
    }

    /**
     * @param  non-empty-string  $seed
     * @return non-empty-string
     */
    private function publicKey(string $seed): string
    {
        return sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair($seed));
    }

    private function trust(string $keyId, string $publicKey): stdClass
    {
        return $this->registry()->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            self::PURPOSE,
            $keyId,
            $publicKey,
        );
    }

    /** @phpstan-impure */
    private function exitCode(string $command): int
    {
        return Artisan::call($command);
    }

    private function registry(): NativeSigningTrustRegistry
    {
        return $this->app->make(NativeSigningTrustRegistry::class);
    }

    private function encode(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }
}
