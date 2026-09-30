<?php

namespace Tests\Feature\GameAuth\NativeLogin;

use App\Accounts\Models\IdentityCanaryAccount;
use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use App\GameAuth\NativeLogin\NativeAdmissionAttempt;
use App\GameAuth\NativeLogin\NativeAdmissionAttempts;
use App\GameAuth\NativeLogin\NativeAdmissionRequest;
use App\GameAuth\NativeLogin\NativeAdmissionResult;
use App\GameAuth\NativeLogin\NativeAdmissionScope;
use App\GameAuth\NativeLogin\NativeAdmissionScopeResolver;
use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\GameAuth\NativeLogin\NativeLoginError;
use App\GameAuth\NativeLogin\NativeLoginRefused;
use App\GameAuth\NativeLogin\RedeemedNativeAccount;
use App\GameAuth\NativeLogin\UnavailableNativeAdmissionScopeResolver;
use App\GameAuth\Tickets\GameLoginTicket;
use App\GameAuth\Tickets\GameLoginTicketDenied;
use App\GameAuth\Tickets\IssueGameLoginTicket;
use App\GameAuth\Tickets\RedeemGameLoginTicket;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\Hash;
use LogicException;
use Tests\TestCase;

final class NativeAdmissionAttemptsTest extends TestCase
{
    use RefreshDatabase;

    private const PURPOSE = 'fresh_admission';

    private const ATTEMPT = '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b';

    private const CHARACTER = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c';

    private const WORLD = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7d';

    private const CHANNEL = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7e';

    private const OTHER_CHANNEL = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7f';

    private string $directory;

    /** @var non-empty-string */
    private string $keySeed;

    private ?NativeAdmissionScope $scope = null;

    protected function setUp(): void
    {
        parent::setUp();

        $this->directory = storage_path('framework/testing/native-login-'.bin2hex(random_bytes(6)));
        mkdir($this->directory.'/witness', 0700, true);
        config([
            'game-auth.native_evidence.high_water_directory' => $this->directory.'/witness',
            'game-auth.native_evidence.fresh_key_purpose' => self::PURPOSE,
            'game-auth.native_evidence.clock_uncertainty_seconds' => 1,
            'game-auth.native_admission.enabled' => true,
            'game-auth.native_admission.grant_ttl_seconds' => 20,
        ]);
        $this->keySeed = $this->installKey('current', 'admission-1');
        $this->trust('admission-1', $this->keySeed);

        $this->scope = $this->scope();
        $this->app->instance(NativeAdmissionScopeResolver::class, new class($this) implements NativeAdmissionScopeResolver
        {
            public function __construct(private readonly NativeAdmissionAttemptsTest $test) {}

            public function resolve(RedeemedNativeAccount $account, NativeAdmissionRequest $request): NativeAdmissionScope
            {
                return $this->test->resolvedScope();
            }
        });
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_000));
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

    public function resolvedScope(): NativeAdmissionScope
    {
        if ($this->scope === null) {
            throw new NativeLoginRefused(NativeLoginError::RouteUnavailable);
        }

        return $this->scope;
    }

    public function test_redemption_binds_the_canonical_account_id_and_consumes_the_ticket_once(): void
    {
        $identity = $this->identity();
        $ticket = $this->ticket($identity);

        $result = $this->admit($this->request($ticket));

        [$header, $payload, $signature] = explode('.', $result->token);
        self::assertTrue(sodium_crypto_sign_verify_detached(
            $this->decode($signature),
            $header.'.'.$payload,
            sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair($this->keySeed)),
        ));
        $claims = json_decode($this->decode($payload), true, 2, JSON_THROW_ON_ERROR);
        self::assertIsArray($claims);
        self::assertSame($identity->account_id, $claims['account_id']);
        self::assertSame(self::ATTEMPT, $claims['attempt_ref']);
        self::assertSame(self::CHARACTER, $claims['character_id']);
        self::assertSame(self::WORLD, $claims['world_id']);
        self::assertSame(self::CHANNEL, $claims['channel_id']);
        self::assertSame('1', $claims['account_security_generation']);
        self::assertSame(20, $result->validForSeconds);
        self::assertSame('rt.3.0123456789abcdef0123456789abcdef', $result->routeRevision);

        $stored = GameLoginTicket::query()->sole();
        self::assertNotNull($stored->used_at);
        self::assertSame(self::ATTEMPT, $stored->attempt_ref);
        $attempt = NativeAdmissionAttempt::query()->sole();
        self::assertSame($header.'.'.$payload, $attempt->signing_input);
        self::assertStringNotContainsString($signature, json_encode($attempt->getAttributes(), JSON_THROW_ON_ERROR));

        $this->assertRefused(NativeLoginError::TicketRejected, $this->request($ticket, attemptRef: '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a70'));
    }

    public function test_retry_returns_a_byte_identical_token_with_remaining_validity(): void
    {
        $ticket = $this->ticket($this->identity());
        $first = $this->admit($this->request($ticket));

        Carbon::setTestNow(Carbon::createFromTimestampMs(1_790_000_007_500));
        $retry = $this->admit($this->request($ticket));

        self::assertSame($first->token, $retry->token);
        self::assertSame(12, $retry->validForSeconds);
        self::assertSame(1, NativeAdmissionAttempt::query()->count());
    }

    public function test_retry_with_any_changed_member_is_an_attempt_conflict_without_mutation(): void
    {
        $identity = $this->identity();
        $ticket = $this->ticket($identity);
        $this->admit($this->request($ticket));
        $otherTicket = $this->ticket($identity);

        $this->assertRefused(NativeLoginError::AttemptConflict, $this->request($otherTicket));
        $this->assertRefused(NativeLoginError::AttemptConflict, $this->request($ticket, characterId: '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a71'));
        $this->assertRefused(NativeLoginError::AttemptConflict, $this->request($ticket, channelId: self::CHANNEL));
        $this->assertRefused(NativeLoginError::AttemptConflict, $this->request($ticket, build: '0.1.1+def456'));

        self::assertNull(GameLoginTicket::query()->where('ticket_hash', hash('sha256', $otherTicket))->sole()->used_at);
        self::assertSame(1, NativeAdmissionAttempt::query()->count());
    }

    public function test_retry_after_expiry_is_expired_even_without_a_signing_input(): void
    {
        $ticket = $this->ticket($this->identity());
        $this->admit($this->request($ticket));

        Carbon::setTestNow(Carbon::createFromTimestampMs(1_790_000_019_001));
        $this->assertRefused(NativeLoginError::GrantExpired, $this->request($ticket));

        NativeAdmissionAttempt::query()->update(['signing_input' => null]);
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_000));
        $this->assertRefused(NativeLoginError::GrantExpired, $this->request($ticket));
    }

    public function test_a_refused_scope_rolls_back_redemption_so_the_same_request_can_retry(): void
    {
        $ticket = $this->ticket($this->identity());
        $this->scope = null;

        $this->assertRefused(NativeLoginError::RouteUnavailable, $this->request($ticket));
        self::assertNull(GameLoginTicket::query()->sole()->used_at);
        self::assertSame(0, NativeAdmissionAttempt::query()->count());

        $this->scope = $this->scope();
        $this->admit($this->request($ticket));
        self::assertNotNull(GameLoginTicket::query()->sole()->used_at);
    }

    public function test_default_resolver_fails_closed_until_route_selection_exists(): void
    {
        $this->app->forgetInstance(NativeAdmissionScopeResolver::class);
        self::assertInstanceOf(UnavailableNativeAdmissionScopeResolver::class, $this->app->make(NativeAdmissionScopeResolver::class));

        $this->assertRefused(NativeLoginError::RouteUnavailable, $this->request($this->ticket($this->identity())));
        self::assertNull(GameLoginTicket::query()->sole()->used_at);
    }

    public function test_scope_must_confirm_the_requested_character_and_channel(): void
    {
        $identity = $this->identity();
        $this->scope = $this->scope(characterId: '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a71');
        $this->assertRefused(NativeLoginError::CharacterConflict, $this->request($this->ticket($identity)));

        $this->scope = $this->scope();
        $this->assertRefused(NativeLoginError::RouteUnavailable, $this->request($this->ticket($identity), channelId: self::OTHER_CHANNEL));

        $this->scope = $this->scope(scopeOwnershipGeneration: '0');
        $this->assertRefused(NativeLoginError::Unavailable, $this->request($this->ticket($identity)));
        self::assertSame(0, GameLoginTicket::query()->whereNotNull('used_at')->count());
    }

    public function test_account_security_changes_between_issuance_and_redemption_deny(): void
    {
        $identity = $this->identity();
        $ticket = $this->ticket($identity);
        $this->app->make(RevokeIdentityGameAuthorizations::class)->execute($identity);
        $this->assertRefused(NativeLoginError::AccountSecurityDenied, $this->request($ticket));

        $identity = $this->identity('disabled@example.test');
        $ticket = $this->ticket($identity);
        $identity->forceFill(['disabled_at' => now()])->save();
        $this->assertRefused(NativeLoginError::AccountSecurityDenied, $this->request($ticket));

        $identity = $this->identity('terminated@example.test');
        $ticket = $this->ticket($identity);
        $identity->forceFill(['terminated_at' => now()])->save();
        $this->assertRefused(NativeLoginError::AccountSecurityDenied, $this->request($ticket));

        self::assertSame(0, GameLoginTicket::query()->whereNotNull('used_at')->count());
    }

    public function test_expired_unknown_and_canary_tickets_are_rejected_and_native_tickets_never_redeem_on_canary(): void
    {
        $identity = $this->identity();
        $ticket = $this->ticket($identity);
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_060));
        $this->assertRefused(NativeLoginError::TicketRejected, $this->request($ticket));
        Carbon::setTestNow(Carbon::createFromTimestamp(1_790_000_000));

        $this->assertRefused(NativeLoginError::TicketRejected, $this->request(str_repeat('a', 43)));

        IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'status' => IdentityCanaryAccount::STATUS_READY,
            'canary_account_id' => 1001,
            'provisioning_name' => 'ready_'.$identity->id,
            'canary_creation_epoch' => 1,
            'ready_at' => now(),
        ]);
        $canary = $this->app->make(IssueGameLoginTicket::class)->execute($identity)->ticket;
        $this->assertRefused(NativeLoginError::TicketRejected, $this->request($canary));

        $native = $this->ticket($identity);
        try {
            $this->app->make(RedeemGameLoginTicket::class)->execute($native, 'oteryn-game-gateway');
            self::fail('A native ticket redeemed on the Canary branch.');
        } catch (GameLoginTicketDenied) {
        }
        self::assertSame(0, GameLoginTicket::query()->whereNotNull('used_at')->count());
    }

    public function test_stored_signing_input_is_re_signed_only_when_bound_to_its_attempt_row(): void
    {
        $identity = $this->identity();
        $ticket = $this->ticket($identity);
        $this->admit($this->request($ticket));
        $otherTicket = $this->ticket($identity);
        $other = '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a70';
        $this->admit($this->request($otherTicket, attemptRef: $other));

        $first = NativeAdmissionAttempt::query()->where('attempt_ref', self::ATTEMPT)->sole();
        $second = NativeAdmissionAttempt::query()->where('attempt_ref', $other)->sole();
        $original = $first->signing_input;
        $first->forceFill(['signing_input' => $second->signing_input])->save();
        $this->assertRefused(NativeLoginError::ReconciliationRequired, $this->request($ticket));

        $first->forceFill(['signing_input' => $original, 'expires_at' => $first->expires_at + 5])->save();
        $this->assertRefused(NativeLoginError::ReconciliationRequired, $this->request($ticket));

        $first->forceFill(['expires_at' => $first->expires_at - 5, 'key_id' => 'admission-2'])->save();
        $this->assertRefused(NativeLoginError::ReconciliationRequired, $this->request($ticket));
    }

    public function test_ticket_consumed_by_the_same_attempt_without_its_record_requires_reconciliation(): void
    {
        $ticket = $this->ticket($this->identity());
        $this->admit($this->request($ticket));
        NativeAdmissionAttempt::query()->delete();

        $this->assertRefused(NativeLoginError::ReconciliationRequired, $this->request($ticket));
        self::assertSame(self::ATTEMPT, GameLoginTicket::query()->sole()->attempt_ref);
    }

    public function test_switch_off_refuses_before_any_redemption(): void
    {
        $ticket = $this->ticket($this->identity());
        config(['game-auth.native_admission.enabled' => false]);

        $this->assertRefused(NativeLoginError::Unavailable, $this->request($ticket));
        self::assertNull(GameLoginTicket::query()->sole()->used_at);
    }

    public function test_request_validation_and_offer_digest(): void
    {
        foreach ([
            fn () => $this->request('ticket', attemptRef: self::ATTEMPT."\n"),
            fn () => $this->request('ticket', attemptRef: strtoupper(self::ATTEMPT)),
            fn () => $this->request('ticket', characterId: '0192b3c4-5d6e-4f80-8a1b-2c3d4e5f6a7c'),
            fn () => $this->request("tick\net"),
            fn () => new NativeAdmissionRequest('ticket', self::ATTEMPT, self::CHARACTER, null, ['client_build' => 'b', 'client_platform' => 'windows']),
            fn () => new NativeAdmissionRequest('ticket', self::ATTEMPT, self::CHARACTER, null, $this->offer(platform: 'linux')),
        ] as $build) {
            try {
                $build();
                self::fail('Malformed native request accepted.');
            } catch (NativeLoginRefused $refused) {
                self::assertSame(NativeLoginError::RequestMalformed, $refused->error);
            }
        }

        try {
            new NativeAdmissionRequest('ticket', self::ATTEMPT, self::CHARACTER, null, $this->offer(alpn: 'h2'));
            self::fail('Unsupported offer accepted.');
        } catch (NativeLoginRefused $refused) {
            self::assertSame(NativeLoginError::OfferUnsupported, $refused->error);
        }

        $reordered = ['transports' => [['alpn' => 'oteryn-game/1', 'transport_profile' => 1, 'protocol_major' => 1]], 'client_platform' => 'windows', 'client_build' => '0.1.0+abc123'];
        self::assertSame(
            hash('sha256', '{"client_build":"0.1.0+abc123","client_platform":"windows","transports":[{"alpn":"oteryn-game/1","protocol_major":1,"transport_profile":1}]}'),
            (new NativeAdmissionRequest('ticket', self::ATTEMPT, self::CHARACTER, null, $reordered))->offerDigest,
        );
        self::assertStringNotContainsString('secret-ticket', print_r($this->request('secret-ticket'), true));
    }

    public function test_request_never_exposes_the_ticket_outside_its_accessor(): void
    {
        $request = $this->request('secret-ticket');

        self::assertSame('secret-ticket', $request->ticket());
        self::assertStringNotContainsString('secret-ticket', json_encode($request, JSON_THROW_ON_ERROR));
        self::assertStringNotContainsString('secret-ticket', var_export($request, true));
        self::assertStringNotContainsString('secret-ticket', print_r($request, true));
        self::assertStringNotContainsString('secret-ticket', print_r(get_object_vars($request), true));

        try {
            $serialized = serialize($request);
            self::fail('Serialization exposed the request: '.$serialized);
        } catch (LogicException $exception) {
            self::assertStringNotContainsString('secret-ticket', $exception->getMessage());
        }
    }

    public function test_error_mapping_follows_contract_section_eleven(): void
    {
        $rows = [];
        foreach (NativeLoginError::cases() as $error) {
            $rows[$error->value] = [$error->httpStatus(), $error->category(), $error->progression(), $error->publicClass(), $error->publicCode()];
        }

        self::assertSame([
            'NATIVE_LOGIN_REQUEST_MALFORMED' => [400, 'INVALID_INPUT', 'TERMINAL', 'RETRY_LOGIN', 'NATIVE_LOGIN_REQUEST_MALFORMED'],
            'NATIVE_LOGIN_UNSUPPORTED_VERSION' => [400, 'UNSUPPORTED_REVISION', 'TERMINAL', 'CLIENT_UPDATE_REQUIRED', 'NATIVE_LOGIN_UNSUPPORTED_VERSION'],
            'NATIVE_LOGIN_OFFER_UNSUPPORTED' => [409, 'UNSUPPORTED_REVISION', 'TERMINAL', 'CLIENT_UPDATE_REQUIRED', 'NATIVE_LOGIN_OFFER_UNSUPPORTED'],
            'NATIVE_LOGIN_TICKET_REJECTED' => [401, 'AUTHENTICATION_FAILED', 'SECURITY_TERMINAL', 'AUTHENTICATION_REQUIRED', 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED'],
            'NATIVE_LOGIN_ACCOUNT_SECURITY_DENIED' => [401, 'SESSION_REJECTED', 'SECURITY_TERMINAL', 'AUTHENTICATION_REQUIRED', 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED'],
            'NATIVE_LOGIN_ATTEMPT_CONFLICT' => [409, 'INVALID_INPUT', 'TERMINAL', 'RETRY_LOGIN', 'NATIVE_LOGIN_ATTEMPT_CONFLICT'],
            'NATIVE_LOGIN_CHARACTER_CONFLICT' => [409, 'CONFLICT', 'TERMINAL', 'SESSION_UNAVAILABLE', 'NATIVE_LOGIN_CHARACTER_CONFLICT'],
            'NATIVE_LOGIN_ROUTE_UNAVAILABLE' => [503, 'DEPENDENCY_UNAVAILABLE', 'RETRYABLE', 'TEMPORARILY_UNAVAILABLE', 'NATIVE_LOGIN_ROUTE_UNAVAILABLE'],
            'NATIVE_LOGIN_RATE_LIMITED' => [429, 'CAPACITY_EXCEEDED', 'RETRYABLE', 'TEMPORARILY_UNAVAILABLE', 'NATIVE_LOGIN_RATE_LIMITED'],
            'ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED' => [503, 'DEPENDENCY_UNAVAILABLE', 'RETRYABLE', 'TEMPORARILY_UNAVAILABLE', 'ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED'],
            'NATIVE_LOGIN_GRANT_EXPIRED' => [409, 'SESSION_REJECTED', 'TERMINAL', 'RETRY_LOGIN', 'NATIVE_LOGIN_GRANT_EXPIRED'],
            'NATIVE_LOGIN_UNAVAILABLE' => [503, 'DEPENDENCY_UNAVAILABLE', 'RETRYABLE', 'TEMPORARILY_UNAVAILABLE', 'NATIVE_LOGIN_UNAVAILABLE'],
        ], $rows);
    }

    private function admit(NativeAdmissionRequest $request): NativeAdmissionResult
    {
        return $this->app->make(NativeAdmissionAttempts::class)->admit($request);
    }

    private function assertRefused(NativeLoginError $expected, NativeAdmissionRequest $request): void
    {
        try {
            $this->admit($request);
            self::fail("Expected {$expected->value}.");
        } catch (NativeLoginRefused $refused) {
            self::assertSame($expected, $refused->error);
        }
    }

    private function request(
        string $ticket,
        string $attemptRef = self::ATTEMPT,
        string $characterId = self::CHARACTER,
        ?string $channelId = null,
        string $build = '0.1.0+abc123',
    ): NativeAdmissionRequest {
        return new NativeAdmissionRequest($ticket, $attemptRef, $characterId, $channelId, $this->offer($build));
    }

    /** @return array<string, mixed> */
    private function offer(string $build = '0.1.0+abc123', string $platform = 'windows', string $alpn = 'oteryn-game/1'): array
    {
        return [
            'client_build' => $build,
            'client_platform' => $platform,
            'transports' => [['protocol_major' => 1, 'transport_profile' => 1, 'alpn' => $alpn]],
        ];
    }

    private function scope(string $characterId = self::CHARACTER, string $scopeOwnershipGeneration = '5'): NativeAdmissionScope
    {
        return new NativeAdmissionScope(
            characterId: $characterId,
            worldId: self::WORLD,
            channelId: self::CHANNEL,
            routeRevision: 'rt.3.0123456789abcdef0123456789abcdef',
            runtimeObservationRevision: 'obs.42',
            scopeOwnershipGeneration: $scopeOwnershipGeneration,
            rulesetRevision: 'ruleset.1',
            contentRevision: 'content.1',
            mapRevision: 'map.1',
            worldPolicyRevision: 'policy.1',
            offerRevision: 'offer.1',
        );
    }

    private function identity(string $email = 'native@example.test'): Identity
    {
        return Identity::query()->create(['email' => $email, 'password' => Hash::make('Correct-Horse-9!Battery')])->refresh();
    }

    /** A native ticket as contract §4.1 defines it; OAuth issuance of this kind waits for U1. */
    private function ticket(Identity $identity): string
    {
        $ticket = bin2hex(random_bytes(32));
        GameLoginTicket::query()->create([
            'ticket_hash' => hash('sha256', $ticket),
            'identity_id' => $identity->id,
            'canary_account_id' => null,
            'account_id' => $identity->account_id,
            'audience' => NativeGameLoginTickets::AUDIENCE,
            'security_generation' => $identity->game_auth_generation,
            'native_security_generation' => $identity->native_security_generation,
            'expires_at' => now()->addSeconds(60),
        ]);

        return $ticket;
    }

    /** @return non-empty-string */
    private function installKey(string $role, string $keyId): string
    {
        $seed = random_bytes(SODIUM_CRYPTO_SIGN_SEEDBYTES);
        $path = $this->directory.'/'.$role.'.key';
        file_put_contents($path, rtrim(strtr(base64_encode($seed), '+/', '-_'), '=')."\n");
        chmod($path, 0600);
        config(['game-auth.native_admission.signing_key_file' => $path, 'game-auth.native_admission.signing_key_id' => $keyId]);

        return $seed;
    }

    /** @param  non-empty-string  $seed */
    private function trust(string $keyId, string $seed): void
    {
        $this->app->make(NativeSigningTrustRegistry::class)->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            self::PURPOSE,
            $keyId,
            sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair($seed)),
        );
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
