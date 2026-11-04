<?php

namespace Tests\Feature\ProductsEntitlements;

use App\Identity\Models\Identity;
use App\ProductsEntitlements\Loyalty\LoyaltyContract;
use App\ProductsEntitlements\Premium\PremiumTimeContract;
use App\ProductsEntitlements\Premium\PremiumTimeLedger;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Illuminate\Testing\TestResponse;
use Tests\TestCase;

final class LoyaltySnapshotEndpointTest extends TestCase
{
    use RefreshDatabase;

    private const PATH = '/internal/v1/products-entitlements/loyalty-snapshots/read';

    private const PREMIUM_PATH = '/internal/v1/products-entitlements/premium-snapshots/read';

    private const PEER = 'CN=oteryn-game-loyalty-snapshot';

    private const PREMIUM_PEER = 'CN=oteryn-game-premium-snapshot';

    private const REVISION = '0123456789abcdef0123456789abcdef01234567';

    private const DAY = 86400;

    private const REASON = 'Operator grant for closed test cohort';

    protected function setUp(): void
    {
        parent::setUp();
        Carbon::setTestNow('2026-10-01T12:00:00Z');
        config([
            'hashing.driver' => 'bcrypt',
            'products-entitlements.loyalty_snapshot.enabled' => true,
            'products-entitlements.loyalty_snapshot.mtls_client_identity' => self::PEER,
            'products-entitlements.loyalty_snapshot.requests_per_minute' => '1200',
            'products-entitlements.loyalty_snapshot.producer_revision' => self::REVISION,
            'products-entitlements.premium_snapshot.enabled' => true,
            'products-entitlements.premium_snapshot.mtls_client_identity' => self::PREMIUM_PEER,
            'products-entitlements.premium_snapshot.requests_per_minute' => '1200',
            'products-entitlements.premium_snapshot.producer_revision' => self::REVISION,
        ]);
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        parent::tearDown();
    }

    public function test_an_account_without_premium_has_zero_points_and_each_read_advances_the_revision(): void
    {
        $account = $this->identity('zero@example.test');

        $first = $this->readLoyalty($account->account_id)->assertOk();
        $this->assertPrivateNoStore($first);
        self::assertSame([
            'schema',
            'producer_revision',
            'nonce',
            'account_id',
            'loyalty_points',
            'loyalty_revision',
            'computed_at',
        ], array_keys($first->json()));
        $first->assertJsonPath('loyalty_points', 0)->assertJsonPath('loyalty_revision', 1);

        $second = $this->readLoyalty($account->account_id)->assertOk();
        $this->assertPrivateNoStore($second);
        $second->assertJsonPath('loyalty_points', 0)->assertJsonPath('loyalty_revision', 2);
        self::assertSame(0, DB::table('premium_time_entitlements')->count());
    }

    public function test_points_count_whole_consumed_premium_days_not_remaining_ones(): void
    {
        $operator = $this->identity('operator-days@example.test');
        $player = $this->identity('player-days@example.test');
        $start = now()->getTimestamp();
        $this->ledger()->grant($operator, $player, 30, self::REASON, $this->requestId());

        $this->assertPoints($player, 0);
        Carbon::setTestNow(Carbon::createFromTimestamp($start + 10 * self::DAY - 1));
        $this->assertPoints($player, 9);
        Carbon::setTestNow(Carbon::createFromTimestamp($start + 10 * self::DAY));
        $this->assertPoints($player, 10);
        Carbon::setTestNow(Carbon::createFromTimestamp($start + 45 * self::DAY));
        $this->assertPoints($player, 30);
    }

    public function test_extensions_and_a_new_interval_after_expiry_are_summed_without_overlap(): void
    {
        $operator = $this->identity('operator-intervals@example.test');
        $player = $this->identity('player-intervals@example.test');
        $start = now()->getTimestamp();
        $this->ledger()->grant($operator, $player, 10, self::REASON, $this->requestId());
        Carbon::setTestNow(Carbon::createFromTimestamp($start + 5 * self::DAY));
        $this->ledger()->grant($operator, $player, 10, self::REASON, $this->requestId());

        Carbon::setTestNow(Carbon::createFromTimestamp($start + 25 * self::DAY));
        $this->assertPoints($player, 20);
        $this->ledger()->grant($operator, $player, 3, self::REASON, $this->requestId());
        Carbon::setTestNow(Carbon::createFromTimestamp($start + 30 * self::DAY));
        $this->assertPoints($player, 23);
    }

    public function test_a_revocation_stops_growth_but_keeps_consumed_days(): void
    {
        $operator = $this->identity('operator-revoke@example.test');
        $player = $this->identity('player-revoke@example.test');
        $start = now()->getTimestamp();
        $this->ledger()->grant($operator, $player, 30, self::REASON, $this->requestId());
        Carbon::setTestNow(Carbon::createFromTimestamp($start + 7 * self::DAY + 3600));
        $this->ledger()->revoke($operator, $player, self::REASON, $this->requestId());

        $this->assertPoints($player, 7);
        Carbon::setTestNow(Carbon::createFromTimestamp($start + 60 * self::DAY));
        $this->assertPoints($player, 7);
    }

    public function test_loyalty_and_premium_reads_share_one_increasing_revision_per_account(): void
    {
        $first = $this->identity('revision-first@example.test');
        $other = $this->identity('revision-other@example.test');

        $loyalty = $this->readLoyalty($first->account_id)->assertOk();
        $this->assertPrivateNoStore($loyalty);
        $loyalty->assertJsonPath('loyalty_revision', 1);

        $premium = $this->readPremium($first->account_id)->assertOk();
        $this->assertPrivateNoStore($premium);
        $premium->assertJsonPath('authority_revision', 2);

        $next = $this->readLoyalty($first->account_id)->assertOk();
        $this->assertPrivateNoStore($next);
        $next->assertJsonPath('loyalty_revision', 3);

        $otherRead = $this->readLoyalty($other->account_id)->assertOk();
        $this->assertPrivateNoStore($otherRead);
        $otherRead->assertJsonPath('loyalty_revision', 1);
    }

    public function test_peer_switch_and_malformed_requests_fail_closed_without_allocating_a_revision(): void
    {
        $account = $this->identity('fail-closed@example.test');
        $body = $this->loyaltyBody($account->account_id, $this->nonce());

        $this->assertFailure($this->rawLoyalty($body, ['SSL_CLIENT_S_DN' => 'CN=wrong']), 401);
        $this->assertFailure($this->rawLoyalty($body, ['SSL_PROTOCOL' => 'TLSv1.2']), 401);

        config(['products-entitlements.loyalty_snapshot.enabled' => null]);
        $this->assertFailure($this->readLoyalty($account->account_id), 503);
        config(['products-entitlements.loyalty_snapshot.enabled' => true]);

        config(['products-entitlements.loyalty_snapshot.mtls_client_identity' => self::PREMIUM_PEER]);
        $this->assertFailure($this->readLoyalty($account->account_id), 503);
        config(['products-entitlements.loyalty_snapshot.mtls_client_identity' => self::PEER]);

        $this->assertFailure($this->rawLoyalty($body, ['CONTENT_TYPE' => 'text/plain']), 400);
        $extra = json_encode([
            'schema' => LoyaltyContract::REQUEST_SCHEMA,
            'account_id' => $account->account_id,
            'nonce' => $this->nonce(),
            'extra' => 'forbidden',
        ], JSON_THROW_ON_ERROR);
        $this->assertFailure($this->rawLoyalty($extra), 400);
        $this->assertFailure($this->rawLoyalty($this->loyaltyBody($account->account_id, 'BAD')), 400);
        $this->assertFailure($this->readLoyalty(strtolower((string) Str::uuid7())), 404);

        self::assertFalse(DB::table('premium_time_authority')->where('identity_id', $account->id)->exists());
    }

    public function test_reads_are_bounded_per_peer(): void
    {
        $account = $this->identity('limited@example.test');
        config(['products-entitlements.loyalty_snapshot.requests_per_minute' => '1']);

        $first = $this->readLoyalty($account->account_id)->assertOk();
        $this->assertPrivateNoStore($first);
        $second = $this->readLoyalty($account->account_id)->assertStatus(429);
        $this->assertPrivateNoStore($second);
        self::assertNotNull($second->headers->get('Retry-After'));
        self::assertSame('', $second->getContent());
    }

    private function assertPoints(Identity $account, int $expected): void
    {
        $response = $this->readLoyalty($account->account_id)->assertOk()->assertJsonPath('loyalty_points', $expected);
        $this->assertPrivateNoStore($response);
    }

    private function ledger(): PremiumTimeLedger
    {
        return app(PremiumTimeLedger::class);
    }

    /** @param array<string, string> $server */
    private function readLoyalty(string $accountId, ?string $nonce = null, array $server = []): TestResponse
    {
        return $this->rawLoyalty($this->loyaltyBody($accountId, $nonce ?? $this->nonce()), $server);
    }

    /** @param array<string, string> $server */
    private function rawLoyalty(string $raw, array $server = []): TestResponse
    {
        return $this->call('POST', self::PATH, [], [], [], array_replace($this->peer(), $server), $raw);
    }

    private function readPremium(string $accountId): TestResponse
    {
        $body = json_encode([
            'schema' => PremiumTimeContract::REQUEST_SCHEMA,
            'account_id' => $accountId,
            'nonce' => $this->nonce(),
        ], JSON_THROW_ON_ERROR);

        return $this->call('POST', self::PREMIUM_PATH, [], [], [], [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PREMIUM_PEER,
        ], $body);
    }

    private function loyaltyBody(string $accountId, string $nonce): string
    {
        return json_encode([
            'schema' => LoyaltyContract::REQUEST_SCHEMA,
            'account_id' => $accountId,
            'nonce' => $nonce,
        ], JSON_THROW_ON_ERROR);
    }

    /** @return array<string, string> */
    private function peer(): array
    {
        return [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PEER,
        ];
    }

    private function assertFailure(TestResponse $response, int $status): void
    {
        $response->assertStatus($status);
        self::assertSame('', $response->getContent());
        $this->assertPrivateNoStore($response);
    }

    private function assertPrivateNoStore(TestResponse $response): void
    {
        $value = (string) $response->headers->get('Cache-Control');
        self::assertStringContainsString('no-store', $value);
        self::assertStringContainsString('private', $value);
    }

    private function nonce(): string
    {
        return bin2hex(random_bytes(16));
    }

    private function requestId(): string
    {
        return strtolower((string) Str::uuid());
    }

    private function identity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }
}
