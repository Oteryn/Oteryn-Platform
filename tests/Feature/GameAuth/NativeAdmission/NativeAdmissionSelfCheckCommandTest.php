<?php

namespace Tests\Feature\GameAuth\NativeAdmission;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use Closure;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Log\Events\MessageLogged;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\Event;
use PHPUnit\Framework\Attributes\DataProvider;
use stdClass;
use Tests\TestCase;

final class NativeAdmissionSelfCheckCommandTest extends TestCase
{
    use RefreshDatabase;

    private const COMMAND = 'game-auth:native-admission:self-check';

    private const PURPOSE = 'fresh_admission';

    private string $directory;

    protected function setUp(): void
    {
        parent::setUp();

        $this->directory = storage_path('framework/testing/native-admission-self-check-'.bin2hex(random_bytes(6)));
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

    public function test_trusted_key_signs_and_verifies_and_prints_only_the_key_id(): void
    {
        Event::fake([MessageLogged::class]);
        $seed = $this->installKey('admission-1');
        $this->trust('admission-1', $this->publicKey($seed));

        self::assertSame(0, Artisan::call(self::COMMAND));
        $output = Artisan::output();

        self::assertSame(['result' => 'ok', 'key_id' => 'admission-1'], json_decode($output, true, 512, JSON_THROW_ON_ERROR));
        $this->assertNoKeyMaterial($seed, $output);
        Event::assertNotDispatched(MessageLogged::class, $this->mentions($seed));
    }

    public function test_retiring_key_does_not_change_which_key_is_checked(): void
    {
        $retiring = $this->installKey('admission-1', 'retiring');
        $current = $this->installKey('admission-2');
        $this->trust('admission-1', $this->publicKey($retiring));
        $this->trust('admission-2', $this->publicKey($current));

        self::assertSame(0, Artisan::call(self::COMMAND));
        self::assertSame(['result' => 'ok', 'key_id' => 'admission-2'], json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR));
    }

    #[DataProvider('refusals')]
    public function test_refusal_exits_one_with_a_reason_and_no_key_material(string $case, string $reason): void
    {
        Event::fake([MessageLogged::class]);
        $seed = $this->installKey('admission-1');
        $this->arrange($case, $seed);

        self::assertSame(1, Artisan::call(self::COMMAND));
        $output = Artisan::output();

        self::assertStringContainsString('Native admission signing self-check failed:', $output);
        self::assertStringContainsString($reason, $output);
        self::assertStringNotContainsString('Exception', $output);
        $this->assertNoKeyMaterial($seed, $output);
        Event::assertNotDispatched(MessageLogged::class, $this->mentions($seed));
    }

    /** @return array<string, array{string, string}> */
    public static function refusals(): array
    {
        return [
            'switch off' => ['switch off', 'issuance is disabled'],
            'production environment' => ['production', 'restricted to testing or preproduction'],
            'key file not configured' => ['no key file', 'current key file must be an absolute path'],
            'key id not configured' => ['no key id', 'current key id is not configured'],
            'key file readable by group' => ['group readable', 'must not be accessible to group or others'],
            'key file holds no seed' => ['no seed', 'canonical base64url 32-byte Ed25519 seed'],
            'kid not trusted' => ['untrusted', 'not trusted for the loaded key'],
            'kid trusted for another public key' => ['other public key', 'not trusted for the loaded key'],
            'kid revoked' => ['revoked', 'not trusted for the loaded key'],
            'key purpose not configured' => ['no key purpose', 'signing key is not configured'],
            'grant TTL out of range' => ['ttl', 'TTL must be 5..30 seconds'],
        ];
    }

    /** @param non-empty-string $seed */
    private function arrange(string $case, string $seed): void
    {
        $trusted = ['revoked', 'no key purpose', 'ttl', 'production', 'switch off'];
        if (in_array($case, $trusted, true)) {
            $this->trust('admission-1', $this->publicKey($seed));
        }

        switch ($case) {
            case 'switch off':
                config(['game-auth.native_admission.enabled' => false]);
                break;
            case 'production':
                $this->app['env'] = 'production';
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
                $this->registry()->revokeKey(
                    NativeEvidenceContract::FRESH_ISSUER,
                    NativeEvidenceContract::FRESH_PROFILE,
                    self::PURPOSE,
                    'admission-1',
                );
                break;
            case 'no key purpose':
                config(['game-auth.native_evidence.fresh_key_purpose' => '']);
                break;
            case 'ttl':
                config(['game-auth.native_admission.grant_ttl_seconds' => 120]);
                break;
        }
    }

    /** @param non-empty-string $seed */
    private function assertNoKeyMaterial(string $seed, string $output): void
    {
        $secretKey = sodium_crypto_sign_secretkey(sodium_crypto_sign_seed_keypair($seed));
        foreach ([$seed, $this->encode($seed), $secretKey, $this->encode($secretKey), bin2hex($seed)] as $material) {
            self::assertStringNotContainsString($material, $output);
        }
        // No compact JWS or signature-sized base64url run reaches the output.
        self::assertDoesNotMatchRegularExpression('/[A-Za-z0-9_-]{40,}/', $output);
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
     * Writes a test-only generated seed and points the given key role at it.
     *
     * @return non-empty-string
     */
    private function installKey(string $keyId, string $role = 'current'): string
    {
        $seed = random_bytes(SODIUM_CRYPTO_SIGN_SEEDBYTES);
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

    private function registry(): NativeSigningTrustRegistry
    {
        return $this->app->make(NativeSigningTrustRegistry::class);
    }

    private function encode(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }
}
