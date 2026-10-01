<?php

namespace Tests\Feature\GameAuth\NativeAdmission;

use App\GameAuth\NativeAdmission\NativeAdmissionGrantContext;
use App\GameAuth\NativeAdmission\NativeAdmissionGrantIssuer;
use App\GameAuth\NativeAdmission\NativeAdmissionKeyring;
use App\GameAuth\NativeAdmission\NativeAdmissionUnavailable;
use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use InvalidArgumentException;
use LogicException;
use stdClass;
use Tests\TestCase;

final class NativeAdmissionGrantIssuerTest extends TestCase
{
    use RefreshDatabase;

    private const PURPOSE = 'fresh_admission';

    private string $directory;

    protected function setUp(): void
    {
        parent::setUp();

        $this->directory = storage_path('framework/testing/native-admission-'.bin2hex(random_bytes(6)));
        mkdir($this->directory.'/witness', 0700, true);
        config([
            'game-auth.native_evidence.high_water_directory' => $this->directory.'/witness',
            'game-auth.native_evidence.fresh_key_purpose' => self::PURPOSE,
            'game-auth.native_admission.enabled' => true,
            'game-auth.native_admission.grant_ttl_seconds' => 20,
        ]);
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        foreach (glob($this->directory.'/{,witness/}*', GLOB_BRACE) ?: [] as $path) {
            (is_link($path) || is_file($path)) && @unlink($path);
        }
        @rmdir($this->directory.'/witness');
        @rmdir($this->directory);

        parent::tearDown();
    }

    public function test_grant_has_exact_header_and_claims_and_verifies_with_the_trusted_key(): void
    {
        $seed = $this->installKey('current', 'admission-1');
        $this->trust('admission-1', $this->publicKey($seed));
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_000));

        $grant = $this->issuer()->issue($this->context());
        [$header, $payload, $signature] = explode('.', $grant->token);

        self::assertSame('{"alg":"Ed25519","kid":"admission-1","typ":"oteryn-admission+jwt"}', $this->decode($header));
        self::assertSame($header.'.'.$payload, $grant->signingInput);
        self::assertTrue(sodium_crypto_sign_verify_detached($this->decode($signature), $grant->signingInput, $this->publicKey($seed)));

        $claims = json_decode($this->decode($payload), true, 3, JSON_THROW_ON_ERROR);
        self::assertIsArray($claims);
        self::assertSame([
            'iss', 'aud', 'iat', 'nbf', 'exp', 'jti', 'profile', 'purpose', 'attempt_ref', 'account_id',
            'character_id', 'world_id', 'channel_id', 'account_security_generation', 'route_revision',
            'runtime_observation_revision', 'scope_ownership_generation', 'protocol_major', 'transport_profile',
            'ruleset_revision', 'content_revision', 'map_revision', 'world_policy_revision', 'offer_revision',
        ], array_keys($claims));
        self::assertSame('urn:oteryn:platform:game-admission', $claims['iss']);
        self::assertSame('urn:oteryn:game:admission', $claims['aud']);
        self::assertSame(1_790_000_000, $claims['iat']);
        self::assertSame($claims['iat'], $claims['nbf']);
        self::assertSame(1_790_000_020, $claims['exp']);
        self::assertSame(1_790_000_020, $grant->expiresAt);
        self::assertIsString($claims['jti']);
        self::assertMatchesRegularExpression('/^[A-Za-z0-9_-]{43}$/', $claims['jti']);
        self::assertSame('oteryn-pre-admission-v1', $claims['profile']);
        self::assertSame('fresh_entry', $claims['purpose']);
        self::assertSame(1, $claims['protocol_major']);
        self::assertSame(1, $claims['transport_profile']);
        self::assertSame('7', $claims['account_security_generation']);
        self::assertSame('rt.3.0123456789abcdef0123456789abcdef', $claims['route_revision']);
        self::assertStringNotContainsString(' ', $this->decode($payload));
        self::assertLessThanOrEqual(4096, strlen($grant->token));

        $second = json_decode($this->decode(explode('.', $this->issuer()->issue($this->context())->token)[1]), true);
        self::assertIsArray($second);
        self::assertNotSame($claims['jti'], $second['jti']);
    }

    public function test_replay_resign_returns_a_byte_identical_token(): void
    {
        $seed = $this->installKey('current', 'admission-1');
        $this->trust('admission-1', $this->publicKey($seed));
        $grant = $this->issuer()->issue($this->context());

        $fresh = $this->app->make(NativeAdmissionGrantIssuer::class);
        self::assertSame($grant->token, $fresh->resign($grant->keyId, $grant->signingInput));
        // @phpstan-ignore staticMethod.alreadyNarrowedType (a second resign must stay byte-identical)
        self::assertSame($grant->token, $fresh->resign($grant->keyId, $grant->signingInput));

        $this->expectException(NativeAdmissionUnavailable::class);
        $fresh->resign('admission-2', $grant->signingInput);
    }

    public function test_untrusted_kid_is_refused(): void
    {
        $this->installKey('current', 'admission-1');

        $this->expectException(NativeAdmissionUnavailable::class);
        $this->issuer()->issue($this->context());
    }

    public function test_kid_trusted_for_another_public_key_is_refused(): void
    {
        $this->installKey('current', 'admission-1');
        $this->trust('admission-1', str_repeat("\x05", 32));

        $this->expectException(NativeAdmissionUnavailable::class);
        $this->issuer()->issue($this->context());
    }

    public function test_revoked_kid_stops_signing_after_the_self_check_cache_expires(): void
    {
        $seed = $this->installKey('current', 'admission-1');
        $this->trust('admission-1', $this->publicKey($seed));
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_000));
        $issuer = $this->issuer();
        $issuer->issue($this->context());

        $this->registry()->revokeKey(NativeEvidenceContract::FRESH_ISSUER, NativeEvidenceContract::FRESH_PROFILE, self::PURPOSE, 'admission-1');
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_005));

        $this->expectException(NativeAdmissionUnavailable::class);
        $issuer->issue($this->context());
    }

    public function test_retiring_key_resigns_while_the_current_key_signs_new_grants(): void
    {
        $old = $this->installKey('retiring', 'admission-1');
        $this->trust('admission-1', $this->publicKey($old));
        config([
            'game-auth.native_admission.signing_key_file' => config('game-auth.native_admission.retiring_signing_key_file'),
            'game-auth.native_admission.signing_key_id' => 'admission-1',
            'game-auth.native_admission.retiring_signing_key_file' => null,
            'game-auth.native_admission.retiring_signing_key_id' => null,
        ]);
        $oldGrant = $this->issuer()->issue($this->context());

        config([
            'game-auth.native_admission.retiring_signing_key_file' => config('game-auth.native_admission.signing_key_file'),
            'game-auth.native_admission.retiring_signing_key_id' => 'admission-1',
        ]);
        $new = $this->installKey('current', 'admission-2');
        $this->trust('admission-2', $this->publicKey($new));

        $issuer = $this->issuer();
        self::assertSame('admission-2', $issuer->issue($this->context())->keyId);
        self::assertSame($oldGrant->token, $issuer->resign('admission-1', $oldGrant->signingInput));
    }

    public function test_disabled_switch_and_invalid_ttl_refuse_signing(): void
    {
        $seed = $this->installKey('current', 'admission-1');
        $this->trust('admission-1', $this->publicKey($seed));

        config(['game-auth.native_admission.grant_ttl_seconds' => 31]);
        try {
            $this->issuer()->issue($this->context());
            self::fail('A TTL above 30 seconds must be refused.');
        } catch (NativeAdmissionUnavailable) {
        }

        config(['game-auth.native_admission.grant_ttl_seconds' => 20, 'game-auth.native_admission.enabled' => false]);
        $this->expectException(NativeAdmissionUnavailable::class);
        $this->issuer()->issue($this->context());
    }

    public function test_key_file_must_be_private_and_hold_one_canonical_seed(): void
    {
        $this->installKey('current', 'admission-1');
        $path = config('game-auth.native_admission.signing_key_file');
        self::assertIsString($path);

        chmod($path, 0640);
        $this->assertKeyringRefuses();

        chmod($path, 0600);
        file_put_contents($path, 'not-a-seed');
        $this->assertKeyringRefuses();

        config(['game-auth.native_admission.signing_key_file' => 'relative/key']);
        $this->assertKeyringRefuses();
    }

    public function test_registry_refuses_rekey_of_a_kid_and_a_third_trusted_key(): void
    {
        $registry = $this->registry();
        $this->trust('admission-1', str_repeat("\x01", 32));
        self::assertEquals(1, $this->trust('admission-1', str_repeat("\x01", 32))->key_revision);

        try {
            $this->trust('admission-1', str_repeat("\x09", 32));
            self::fail('Re-keying a fresh admission kid must be refused.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('bound to one public key', $exception->getMessage());
        }

        $this->trust('admission-2', str_repeat("\x02", 32));
        try {
            $this->trust('admission-3', str_repeat("\x03", 32));
            self::fail('A third trusted fresh admission key must be refused.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('At most two', $exception->getMessage());
        }

        $registry->revokeKey(NativeEvidenceContract::FRESH_ISSUER, NativeEvidenceContract::FRESH_PROFILE, self::PURPOSE, 'admission-1');
        $this->trust('admission-3', str_repeat("\x03", 32));

        try {
            $this->trust('admission-1', str_repeat("\x01", 32));
            self::fail('A revoked kid must never be re-trusted.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('cannot be re-trusted', $exception->getMessage());
        }

        self::assertNull($registry->trustedPublicKey(NativeEvidenceContract::FRESH_ISSUER, NativeEvidenceContract::FRESH_PROFILE, self::PURPOSE, 'admission-1'));
        self::assertSame(
            NativeEvidenceContract::encodePublicKey(str_repeat("\x03", 32)),
            $registry->trustedPublicKey(NativeEvidenceContract::FRESH_ISSUER, NativeEvidenceContract::FRESH_PROFILE, self::PURPOSE, 'admission-3'),
        );
    }

    public function test_context_rejects_non_canonical_identifiers_and_revisions(): void
    {
        foreach ([
            ['attemptRef' => '0192B3C4-5D6E-7F80-9A1B-2C3D4E5F6A7B'],
            ['channelId' => '0192b3c4-5d6e-4f80-9a1b-2c3d4e5f6a7b'],
            ['accountSecurityGeneration' => '0'],
            ['scopeOwnershipGeneration' => '18446744073709551616'],
            ['routeRevision' => 'rt.1."x"'],
            ['offerRevision' => str_repeat('a', 65)],
        ] as $override) {
            try {
                $this->context($override);
                self::fail('Context must reject '.json_encode($override));
            } catch (InvalidArgumentException) {
            }
        }
        self::assertSame('18446744073709551615', $this->context(['scopeOwnershipGeneration' => '18446744073709551615'])->scopeOwnershipGeneration);
    }

    public function test_resign_refuses_a_tampered_expired_or_oversized_stored_payload(): void
    {
        $seed = $this->installKey('current', 'admission-1');
        $this->trust('admission-1', $this->publicKey($seed));
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_000));
        $issuer = $this->issuer();
        $grant = $issuer->issue($this->context());
        $claims = json_decode($this->decode(explode('.', $grant->signingInput)[1]), true, 2, JSON_THROW_ON_ERROR);
        self::assertIsArray($claims);
        $iat = $claims['iat'] ?? null;
        self::assertIsInt($iat);

        $tampered = [
            'extra claim' => json_encode($claims + ['admin' => true], JSON_UNESCAPED_SLASHES),
            'missing claim' => json_encode(array_diff_key($claims, ['offer_revision' => true]), JSON_UNESCAPED_SLASHES),
            'reordered claims' => json_encode(['aud' => $claims['aud'], 'iss' => $claims['iss']] + $claims, JSON_UNESCAPED_SLASHES),
            'wrong issuer' => json_encode(['iss' => 'urn:oteryn:platform:game-recovery'] + $claims, JSON_UNESCAPED_SLASHES),
            'wrong audience' => json_encode(array_replace($claims, ['aud' => 'urn:oteryn:game:other']), JSON_UNESCAPED_SLASHES),
            'wrong profile' => json_encode(array_replace($claims, ['profile' => 'oteryn-reauth-recovery-v1']), JSON_UNESCAPED_SLASHES),
            'wrong purpose' => json_encode(array_replace($claims, ['purpose' => 'platform_security']), JSON_UNESCAPED_SLASHES),
            'nbf differs from iat' => json_encode(array_replace($claims, ['nbf' => $iat + 1]), JSON_UNESCAPED_SLASHES),
            'ttl above 30 s' => json_encode(array_replace($claims, ['exp' => $iat + 31]), JSON_UNESCAPED_SLASHES),
            'iat in the future' => json_encode(array_replace($claims, [
                'iat' => $iat + 157_680_000,
                'nbf' => $iat + 157_680_000,
                'exp' => $iat + 157_680_030,
            ]), JSON_UNESCAPED_SLASHES),
            'iat one second ahead' => json_encode(array_replace($claims, ['iat' => $iat + 1, 'nbf' => $iat + 1, 'exp' => $iat + 21]), JSON_UNESCAPED_SLASHES),
            'string generation with newline' => json_encode(array_replace($claims, ['account_security_generation' => "7\n"]), JSON_UNESCAPED_SLASHES),
            'nested claim' => json_encode(array_replace($claims, ['route_revision' => ['rt']]), JSON_UNESCAPED_SLASHES),
            'whitespace' => json_encode($claims, JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT),
            'not json' => 'not json',
            'oversized' => json_encode($claims + ['pad' => str_repeat('a', 3072)], JSON_UNESCAPED_SLASHES),
        ];
        foreach ($tampered as $case => $payload) {
            self::assertIsString($payload);
            try {
                $issuer->resign('admission-1', $this->signingInput('admission-1', $payload));
                self::fail("Resign must refuse a stored payload with: {$case}.");
            } catch (NativeAdmissionUnavailable) {
                self::addToAssertionCount(1);
            }
        }

        // Exactly 1 s left is re-signable (contract §6.2).
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_019));
        self::assertSame($grant->token, $issuer->resign('admission-1', $grant->signingInput));

        Carbon::setTestNow(Carbon::createFromTimestampMs(1_790_000_019_500));
        try {
            $issuer->resign('admission-1', $grant->signingInput);
            self::fail('Resign must refuse a grant with less than 1 s left.');
        } catch (NativeAdmissionUnavailable) {
            self::addToAssertionCount(1);
        }

        // Exactly 0 s left.
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_020));
        $this->expectException(NativeAdmissionUnavailable::class);
        $issuer->resign('admission-1', $grant->signingInput);
    }

    public function test_production_environment_is_refused(): void
    {
        $seed = $this->installKey('current', 'admission-1');
        $this->trust('admission-1', $this->publicKey($seed));
        $grant = $this->issuer()->issue($this->context());

        $this->app['env'] = 'production';
        try {
            $this->issuer()->issue($this->context());
            self::fail('Issuance must be refused in production.');
        } catch (NativeAdmissionUnavailable $exception) {
            self::assertStringContainsString('testing or preproduction', $exception->getMessage());
        }

        $this->expectException(NativeAdmissionUnavailable::class);
        $this->issuer()->resign($grant->keyId, $grant->signingInput);
    }

    public function test_resign_is_refused_when_the_switch_is_off(): void
    {
        $seed = $this->installKey('current', 'admission-1');
        $this->trust('admission-1', $this->publicKey($seed));
        $grant = $this->issuer()->issue($this->context());

        config(['game-auth.native_admission.enabled' => false]);
        $this->expectException(NativeAdmissionUnavailable::class);
        $this->expectExceptionMessage('disabled');
        $this->issuer()->resign($grant->keyId, $grant->signingInput);
    }

    public function test_revoked_profile_stops_signing(): void
    {
        $seed = $this->installKey('current', 'admission-1');
        $this->trust('admission-1', $this->publicKey($seed));
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_000));
        $grant = $this->issuer()->issue($this->context());

        $this->registry()->revokeProfile(NativeEvidenceContract::FRESH_ISSUER, NativeEvidenceContract::FRESH_PROFILE, self::PURPOSE);

        try {
            $this->issuer()->issue($this->context());
            self::fail('A revoked trust profile must stop issuance.');
        } catch (NativeAdmissionUnavailable) {
            self::addToAssertionCount(1);
        }
        $this->expectException(NativeAdmissionUnavailable::class);
        $this->issuer()->resign($grant->keyId, $grant->signingInput);
    }

    public function test_symlinked_key_file_and_retiring_kid_equal_to_current_are_refused(): void
    {
        $this->installKey('current', 'admission-1');
        $path = config('game-auth.native_admission.signing_key_file');
        self::assertIsString($path);

        $link = $this->directory.'/link.key';
        self::assertTrue(symlink($path, $link));
        config(['game-auth.native_admission.signing_key_file' => $link]);
        $this->assertKeyringRefuses();

        config(['game-auth.native_admission.signing_key_file' => $this->directory]);
        $this->assertKeyringRefuses();

        config(['game-auth.native_admission.signing_key_file' => $path]);
        $this->installKey('retiring', 'admission-1');
        try {
            (new NativeAdmissionKeyring)->currentKeyId();
            self::fail('A retiring kid equal to the current kid must be refused.');
        } catch (NativeAdmissionUnavailable $exception) {
            self::assertStringContainsString('must differ', $exception->getMessage());
        }
    }

    public function test_trailing_newline_is_refused_everywhere(): void
    {
        foreach ([
            ['attemptRef' => "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b\n"],
            ['accountId' => "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a70\n"],
            ['accountSecurityGeneration' => "7\n"],
            ['scopeOwnershipGeneration' => "5\n"],
            ['routeRevision' => "rt.1\n"],
            ['offerRevision' => "offer.1\n"],
        ] as $override) {
            try {
                $this->context($override);
                self::fail('Context must reject '.json_encode($override));
            } catch (InvalidArgumentException) {
                self::addToAssertionCount(1);
            }
        }

        try {
            $this->trust("admission-1\n", str_repeat("\x01", 32));
            self::fail('The registry must refuse a kid with a trailing newline.');
        } catch (InvalidArgumentException) {
            self::addToAssertionCount(1);
        }

        $seed = $this->installKey('current', "admission-1\n");
        $this->assertKeyringRefuses();

        config(['game-auth.native_admission.signing_key_id' => 'admission-1']);
        $this->trust('admission-1', $this->publicKey($seed));
        $grant = $this->issuer()->issue($this->context());
        try {
            $this->issuer()->resign("admission-1\n", $grant->signingInput);
            self::fail('Resign must refuse a kid with a trailing newline.');
        } catch (NativeAdmissionUnavailable) {
            self::addToAssertionCount(1);
        }

        $this->expectException(NativeAdmissionUnavailable::class);
        $this->issuer()->resign('admission-1', $grant->signingInput."\n");
    }

    public function test_registry_refuses_cross_version_reuse_of_a_kid(): void
    {
        $registry = $this->registry();
        $keyA = str_repeat("\x01", 32);
        $keyB = str_repeat("\x09", 32);
        $this->trust('admission-1', $keyA);
        $registry->revokeKey(NativeEvidenceContract::FRESH_ISSUER, NativeEvidenceContract::FRESH_PROFILE, self::PURPOSE, 'admission-1');
        $registry->revokeProfile(NativeEvidenceContract::FRESH_ISSUER, NativeEvidenceContract::FRESH_PROFILE, self::PURPOSE);
        $registry->publishNextProfileVersion(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            self::PURPOSE,
            'admission-2',
            str_repeat("\x02", 32),
        );

        foreach (['same key' => $keyA, 'another key' => $keyB] as $case => $key) {
            try {
                $this->trust('admission-1', $key);
                self::fail("Re-publishing a kid of an earlier profile version with the {$case} must be refused.");
            } catch (LogicException $exception) {
                self::assertStringContainsString('cannot be reused across trust profile versions', $exception->getMessage());
            }
        }

        self::assertNull($registry->trustedPublicKey(NativeEvidenceContract::FRESH_ISSUER, NativeEvidenceContract::FRESH_PROFILE, self::PURPOSE, 'admission-1'));
        self::assertSame(
            NativeEvidenceContract::encodePublicKey(str_repeat("\x03", 32)),
            $this->trust('admission-3', str_repeat("\x03", 32))->public_key,
        );
    }

    public function test_keyring_redacts_debug_output_and_refuses_serialization(): void
    {
        $seed = $this->installKey('current', 'admission-1');
        $keyring = new NativeAdmissionKeyring;
        self::assertSame('admission-1', $keyring->currentKeyId());
        $secretKey = sodium_crypto_sign_secretkey(sodium_crypto_sign_seed_keypair($seed));

        $dump = print_r($keyring, true);
        self::assertStringContainsString('admission-1', $dump);
        foreach ([
            $dump,
            var_export($keyring, true),
            var_export((array) $keyring, true),
            print_r((array) $keyring, true),
            (string) json_encode((array) $keyring),
        ] as $exported) {
            self::assertStringNotContainsString($secretKey, $exported);
            self::assertStringNotContainsString($seed, $exported);
            self::assertStringNotContainsString(substr($secretKey, 0, 16), $exported);
        }
        self::assertSame(['currentKeyId' => 'admission-1', 'loadedKeyIds' => ['admission-1']], $keyring->__debugInfo());

        $this->expectException(LogicException::class);
        self::assertNotSame('', serialize($keyring));
    }

    private function assertKeyringRefuses(): void
    {
        try {
            (new NativeAdmissionKeyring)->currentKeyId();
            self::fail('Keyring must refuse the key file.');
        } catch (NativeAdmissionUnavailable) {
            self::addToAssertionCount(1);
        }
    }

    /**
     * Writes a test-only generated seed and points the given key role at it.
     *
     * @return non-empty-string
     */
    private function installKey(string $role, string $keyId): string
    {
        $seed = random_bytes(SODIUM_CRYPTO_SIGN_SEEDBYTES);
        $path = $this->directory.'/'.$role.'.key';
        file_put_contents($path, rtrim(strtr(base64_encode($seed), '+/', '-_'), '=')."\n");
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

    private function issuer(): NativeAdmissionGrantIssuer
    {
        return $this->app->make(NativeAdmissionGrantIssuer::class);
    }

    /** @param array<string, string> $override */
    private function context(array $override = []): NativeAdmissionGrantContext
    {
        return new NativeAdmissionGrantContext(...array_merge([
            'attemptRef' => '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b',
            'accountId' => '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a70',
            'characterId' => '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c',
            'worldId' => '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7d',
            'channelId' => '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7e',
            'accountSecurityGeneration' => '7',
            'routeRevision' => 'rt.3.0123456789abcdef0123456789abcdef',
            'runtimeObservationRevision' => 'obs.42',
            'scopeOwnershipGeneration' => '5',
            'rulesetRevision' => 'ruleset.1',
            'contentRevision' => 'content.1',
            'mapRevision' => 'map.1',
            'worldPolicyRevision' => 'policy.1',
            'offerRevision' => 'offer.1',
        ], $override));
    }

    private function signingInput(string $keyId, string $payload): string
    {
        $encode = static fn (string $bytes): string => rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');

        return $encode('{"alg":"Ed25519","kid":"'.$keyId.'","typ":"oteryn-admission+jwt"}').'.'.$encode($payload);
    }

    /** @return non-empty-string */
    private function decode(string $segment): string
    {
        $decoded = base64_decode(strtr($segment, '-_', '+/'), true);
        if (! is_string($decoded) || $decoded === '') {
            self::fail('Segment is not base64url.');
        }

        return $decoded;
    }
}
