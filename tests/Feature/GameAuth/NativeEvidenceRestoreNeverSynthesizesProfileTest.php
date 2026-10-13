<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeEvidenceNamespace;
use App\GameAuth\NativeEvidence\NativeEvidencePublicationFence;
use App\GameAuth\NativeEvidence\NativeEvidenceUnavailable;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use Closure;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use LogicException;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\Feature\GameAuth\Concerns\ReadsJsonOutput;
use Tests\Feature\GameAuth\Concerns\WritesAttemptEvidence;
use Tests\TestCase;
use Throwable;

/**
 * PostgreSQL P6a negative test: a database restore (A: profile and key rows gone, witness kept; B: A plus
 * the witness-store row gone) never lets any entry point create a profile or touch the witness floor.
 */
final class NativeEvidenceRestoreNeverSynthesizesProfileTest extends TestCase
{
    use ReadsJsonOutput;
    use RefreshDatabase;
    use WritesAttemptEvidence;

    private string $root;

    private string $witnessDirectory;

    private string $evidenceDirectory;

    private string $storeId;

    private string $floorBytes;

    private string $markerBytes;

    protected function setUp(): void
    {
        parent::setUp();

        $this->root = storage_path('framework/testing/native-restore-negative-'.bin2hex(random_bytes(6)));
        $this->witnessDirectory = $this->root.'/witness';
        $this->evidenceDirectory = $this->root.'/evidence';
        mkdir($this->witnessDirectory, 0700, true);
        mkdir($this->evidenceDirectory, 0700, true);
        config([
            'game-auth.native_evidence.high_water_directory' => $this->witnessDirectory,
            'game-auth.native_evidence.fresh_key_purpose' => self::FIXTURE_PURPOSE,
            'game-auth.native_evidence.first_publication_fence' => 'off',
            'game-auth.native_evidence.attempt_evidence_directory' => $this->evidenceDirectory,
            'game-auth.native_admission.enabled' => true,
            'game-auth.native_admission.retiring_signing_key_file' => null,
            'game-auth.native_admission.retiring_signing_key_id' => null,
        ]);
    }

    protected function tearDown(): void
    {
        foreach (glob($this->root.'/{witness,evidence}/*', GLOB_BRACE) ?: [] as $path) {
            @unlink($path);
        }
        foreach (glob($this->root.'/*.key') ?: [] as $path) {
            @unlink($path);
        }
        @rmdir($this->witnessDirectory);
        @rmdir($this->evidenceDirectory);
        @rmdir($this->root);

        parent::tearDown();
    }

    #[DataProvider('restores')]
    public function test_no_entry_point_synthesizes_a_profile_after_a_restore(string $restore): void
    {
        $seedOne = random_bytes(SODIUM_CRYPTO_SIGN_SEEDBYTES);
        $seedTwo = random_bytes(SODIUM_CRYPTO_SIGN_SEEDBYTES);
        $this->installKey('kid-1', $seedOne);

        // Arrange: provenance by the init, then a real publication that commits for kid-1.
        self::assertSame(0, Artisan::call('game-auth:native-evidence:init-provenance'));
        $this->storeId = self::stringAt(self::jsonOutput(), 'store_id');
        $registry = app(NativeSigningTrustRegistry::class);
        $registry->publishTrustedKey(...$this->scope(), ...['kid-1', $this->publicKey($seedOne)]);
        self::assertSame(1, DB::table('native_game_signing_trust_profiles')->count());
        self::assertSame(1, DB::table('native_game_signing_trust_key_versions')->count());
        $this->floorBytes = (string) file_get_contents($this->floorPath());
        $this->markerBytes = (string) file_get_contents($this->witnessDirectory.'/witness-store.id');
        self::assertSame("1\n", $this->floorBytes);

        // Restore A: the publication is rolled back, the witness is kept. Restore B: also before the init.
        DB::table('native_game_signing_trust_key_versions')->delete();
        DB::table('native_game_signing_trust_profiles')->delete();
        if ($restore === 'B') {
            DB::table('native_game_evidence_witness_stores')->delete();
        }
        $this->assertUnsynthesized();

        $hashes = $this->writeRun($this->evidenceDirectory, self::FIXTURE_RUN, $this->standardRecords($this->storeId, [
            'intent' => ['key_id' => 'kid-2'],
        ]));
        $fence = new NativeEvidencePublicationFence(self::FIXTURE_RUN, $hashes[4]);
        $encodedFence = rtrim(strtr(base64_encode((string) json_encode(['run_id' => self::FIXTURE_RUN, 'intent_sha256' => $hashes[4]])), '+/', '-_'), '=');

        // Act. The init and the observation run first: every runtime witness call re-binds a missing row.
        self::assertSame($restore === 'A' ? 0 : 1, Artisan::call('game-auth:native-evidence:init-provenance'));
        if ($restore === 'A') {
            self::assertSame('unchanged', self::at(self::jsonOutput(), 'result'));
        }
        $this->assertUnsynthesized();

        self::assertSame(0, Artisan::call('game-auth:native-evidence:observe-trust', ['--trust' => 'fresh', '--key-id' => 'kid-1']));
        $observed = $this->decoded();
        self::assertSame($restore === 'A' ? 'match' : 'marker_only', $observed['store']['binding']);
        self::assertFalse($observed['profile']['present']);
        $this->assertUnsynthesized();

        $writers = [
            'publish kid-1, fence off' => fn () => $registry->publishTrustedKey(...$this->scope(), ...['kid-1', $this->publicKey($seedOne)]),
            'publish kid-2, fence off' => fn () => $registry->publishTrustedKey(...$this->scope(), ...['kid-2', $this->publicKey($seedTwo)]),
            'publish kid-1, fence required, no fence' => function () use ($registry, $seedOne): void {
                config(['game-auth.native_evidence.first_publication_fence' => 'required']);
                $registry->publishTrustedKey(...$this->scope(), ...['kid-1', $this->publicKey($seedOne)]);
            },
            'publish kid-2, fence required, open intent' => function () use ($registry, $seedTwo, $fence): void {
                config(['game-auth.native_evidence.first_publication_fence' => 'required']);
                $registry->publishTrustedKey(...$this->scope(), ...['kid-2', $this->publicKey($seedTwo), $fence]);
            },
            'next profile version' => fn () => $registry->publishNextProfileVersion(...$this->scope(), ...['kid-3', $this->publicKey($seedTwo)]),
            'revoke key' => fn () => $registry->revokeKey(...$this->scope(), ...['kid-1']),
            'revoke profile' => fn () => $registry->revokeProfile(...$this->scope()),
        ];
        foreach ($writers as $name => $writer) {
            $this->assertWriterRefuses($name, $writer);
            $this->assertUnsynthesized();
        }
        config(['game-auth.native_evidence.first_publication_fence' => 'off']);

        $commands = [
            'publish-trust kid-1' => fn () => $this->publishTrustCommand('kid-1', $seedOne, null),
            'publish-trust kid-2' => fn () => $this->publishTrustCommand('kid-2', $seedTwo, null),
            'publish-trust kid-2, required, open intent' => function () use ($seedTwo, $encodedFence): int {
                config(['game-auth.native_evidence.first_publication_fence' => 'required']);

                return $this->publishTrustCommand('kid-2', $seedTwo, $encodedFence);
            },
            'publish-key kid-2' => function () use ($seedTwo): int {
                $path = $this->root.'/kid-2.pub';
                file_put_contents($path, $this->publicKey($seedTwo));

                return Artisan::call('game-auth:native-trust:publish-key', ['--key-id' => 'kid-2', '--public-key-file' => $path]);
            },
            'reconcile --trust=fresh' => fn () => Artisan::call('game-auth:native-evidence:reconcile', ['--trust' => 'fresh']),
        ];
        foreach ($commands as $name => $command) {
            self::assertSame(1, $command(), $name.' must fail closed.');
            $this->assertUnsynthesized();
        }
        @unlink($this->root.'/kid-2.pub');

        // Runtime re-bind (restore B): provenance comes back from the marker, trust does not.
        self::assertSame($this->storeId, DB::table('native_game_evidence_witness_stores')->where('id', 1)->value('store_id'));
        self::assertSame(0, DB::table('native_game_signing_trust_profiles')->count());
        self::assertSame(0, DB::table('native_game_signing_trust_key_versions')->count());
    }

    /** @return array<string, array{string}> */
    public static function restores(): array
    {
        return [
            'A: profile and key rows lost, witness kept' => ['A'],
            'B: also the witness-store row lost' => ['B'],
        ];
    }

    private function assertWriterRefuses(string $name, Closure $writer): void
    {
        try {
            $writer();
        } catch (NativeEvidenceUnavailable|LogicException) {
            return;
        } catch (Throwable $exception) {
            self::fail($name.' threw '.$exception::class.' instead of failing closed.');
        }

        self::fail($name.' must not succeed.');
    }

    private function assertUnsynthesized(): void
    {
        self::assertSame(0, DB::table('native_game_signing_trust_profiles')->count());
        self::assertSame(0, DB::table('native_game_signing_trust_key_versions')->count());
        self::assertSame($this->floorBytes, file_get_contents($this->floorPath()));
        self::assertSame($this->markerBytes, file_get_contents($this->witnessDirectory.'/witness-store.id'));
        self::assertSame([$this->floorPath()], glob($this->witnessDirectory.'/*.floor') ?: []);
        self::assertSame([], glob($this->witnessDirectory.'/*.floor.tmp.*') ?: []);
        self::assertSame([], glob($this->witnessDirectory.'/witness-store.id.tmp.*') ?: []);
    }

    private function publishTrustCommand(string $keyId, string $seed, ?string $fence): int
    {
        $this->installKey($keyId, $seed);

        return Artisan::call(
            'game-auth:native-admission:publish-trust',
            $fence === null ? [] : ['--first-publication-fence' => $fence],
        );
    }

    /** @return array{string, string, string} */
    private function scope(): array
    {
        return [NativeEvidenceContract::FRESH_ISSUER, NativeEvidenceContract::FRESH_PROFILE, self::FIXTURE_PURPOSE];
    }

    private function floorPath(): string
    {
        return $this->witnessDirectory.'/'.NativeEvidenceNamespace::trustState(...$this->scope()).'.floor';
    }

    private function installKey(string $keyId, string $seed): void
    {
        $path = $this->root.'/current.key';
        file_put_contents($path, rtrim(strtr(base64_encode($seed), '+/', '-_'), '=')."\n");
        chmod($path, 0600);
        config([
            'game-auth.native_admission.signing_key_file' => $path,
            'game-auth.native_admission.signing_key_id' => $keyId,
        ]);
    }

    /** @return array{result: string, store_id: string, store: array{binding: string}, profile: array{present: bool}} */
    private function decoded(): array
    {
        /** @var array{result: string, store_id: string, store: array{binding: string}, profile: array{present: bool}} $decoded */
        $decoded = json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR);

        return $decoded;
    }

    private function publicKey(string $seed): string
    {
        assert($seed !== '');

        return sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair($seed));
    }
}
