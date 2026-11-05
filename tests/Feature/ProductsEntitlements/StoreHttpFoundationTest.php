<?php

namespace Tests\Feature\ProductsEntitlements;

use App\ProductsEntitlements\Store\StoreSettings;
use Illuminate\Cache\RateLimiter;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Testing\TestResponse;
use PHPUnit\Framework\Attributes\DataProvider;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

final class StoreHttpFoundationTest extends TestCase
{
    use RefreshDatabase;

    private const PEER = 'CN=store-test-peer';

    private const SOURCE = 'game.store.test';

    /** @var array<string, array{string, int}> */
    private const ROUTES = [
        'balance/read' => ['ReadStoreBalanceV1', 512],
        'history/read' => ['ReadStoreHistoryPageV1', 512],
        'purchases' => ['PurchaseStoreOfferV1', 1024],
        'operations/pending' => ['ListPendingStoreOperationsV1', 512],
        'operations/read' => ['ReadStoreOperationIntentV1', 512],
        'operations/receipt' => ['PublishStoreDeliveryReceiptV1', 1024],
        'operations/reconcile' => ['ListStoreOperationStatesV1', 512],
    ];

    protected function setUp(): void
    {
        parent::setUp();
        config(['products-entitlements.store.purchase_enabled' => true,
            'products-entitlements.store.delivery_enabled' => true,
            'products-entitlements.store.mtls_client_identity' => self::PEER,
            'products-entitlements.store.source_authority' => self::SOURCE]);
    }

    public function test_store_defaults_and_each_disabled_route_fail_closed_before_authentication(): void
    {
        $defaults = require base_path('config/products-entitlements.php');
        self::assertIsArray($defaults);
        self::assertIsArray($defaults['store']);
        self::assertFalse($defaults['store']['purchase_enabled']);
        self::assertFalse($defaults['store']['delivery_enabled']);
        self::assertSame([], $defaults['store']['enabled_unit_kinds']);
        config(['products-entitlements.store.purchase_enabled' => false, 'products-entitlements.store.delivery_enabled' => false]);
        foreach (self::ROUTES as $path => [$operation]) {
            $this->emptyStatus($this->send($path, $this->body($operation), ['SSL_CLIENT_VERIFY' => 'NONE']), 503);
        }
        self::assertSame(0, DB::table('store_purchase_requests')->count());
    }

    public function test_head_only_requests_are_rejected_by_implemented_handlers_without_writes(): void
    {
        foreach (self::ROUTES as $path => [$operation]) {
            $this->emptyStatus($this->send($path, $this->body($operation)), str_starts_with($path, 'operations/') ? 503 : 400);
        }
        foreach (['store_purchase_requests', 'store_orders', 'store_operations', 'store_receipts'] as $table) {
            self::assertSame(0, DB::table($table)->count());
        }
    }

    /** @return iterable<string, array{string, mixed}> */
    public static function badSettings(): iterable
    {
        yield 'purchase without delivery' => ['delivery_enabled', false];
        yield 'malformed purchase flag' => ['purchase_enabled', 'true'];
        yield 'malformed delivery flag' => ['delivery_enabled', 1];
        foreach ([null, '', str_repeat('x', 129), "CN=peer\n"] as $i => $value) {
            yield 'identity '.$i => ['mtls_client_identity', $value];
        }
        foreach ([null, '', 'bad authority', 4] as $i => $value) {
            yield 'source '.$i => ['source_authority', $value];
        }
        foreach (['requests_per_minute' => 6000, 'purchases_per_account_per_minute' => 60,
            'reads_per_account_per_minute' => 600, 'max_pending_operations' => 100000] as $key => $max) {
            foreach ([0, $max + 1, null, 1.5, '01', 'one'] as $i => $value) {
                yield $key.' '.$i => [$key, $value];
            }
        }
        foreach (['{', '{}', '["COIN"]', '["ITEM","ITEM"]', '[1]', null] as $i => $value) {
            yield 'kinds '.$i => ['enabled_unit_kinds', $value];
        }
    }

    #[DataProvider('badSettings')]
    public function test_store_misconfiguration_is_503_on_every_route(string $key, mixed $value): void
    {
        config(['products-entitlements.store.'.$key => $value]);
        foreach (self::ROUTES as $path => [$operation]) {
            $this->emptyStatus($this->send($path, $this->body($operation), ['SSL_CLIENT_VERIFY' => 'NONE']), 503);
        }
    }

    public function test_store_configuration_boundaries_and_environment_limit(): void
    {
        config(['products-entitlements.store.requests_per_minute' => '6000',
            'products-entitlements.store.purchases_per_account_per_minute' => '60',
            'products-entitlements.store.reads_per_account_per_minute' => '600',
            'products-entitlements.store.max_pending_operations' => '100000',
            'products-entitlements.store.enabled_unit_kinds' => '["PREMIUM_TIME","COSMETIC","ITEM"]']);
        $settings = StoreSettings::current('purchase');
        self::assertNotNull($settings);
        self::assertSame(100000, $settings->maxPendingOperations);
        self::assertSame(['PREMIUM_TIME', 'COSMETIC', 'ITEM'], $settings->enabledUnitKinds);
        foreach (['preproduction', 'production', 'local'] as $env) {
            $this->app->instance('env', $env);
            $this->emptyStatus($this->send('balance/read', $this->body('ReadStoreBalanceV1'), ['SSL_PROTOCOL' => 'TLSv1.2']), $env === 'preproduction' ? 401 : 503);
        }
    }

    /** @return iterable<string, array{string, mixed}> */
    public static function overlappingPurposes(): iterable
    {
        foreach (['game-auth.native_evidence.mtls_client_identity', 'game-auth.character_bootstrap_intent.mtls_client_identity',
            'products-entitlements.premium_snapshot.mtls_client_identity'] as $key) {
            yield $key => [$key, self::PEER];
        }
        foreach (['game-auth.native_account_characters.identities', 'character-authority.commands.identities',
            'character-authority.restore.identities', 'game-projections.public.identities'] as $key) {
            yield $key => [$key, json_encode([self::PEER], JSON_THROW_ON_ERROR)];
        }
        foreach (['game-auth.native_runtime_status.identities', 'game-auth.native_scope_assignment.identities'] as $key) {
            yield $key => [$key, [self::PEER => ['scope']]];
        }
    }

    #[DataProvider('overlappingPurposes')]
    public function test_store_identity_overlapping_any_other_purpose_is_unavailable(string $key, mixed $value): void
    {
        config([$key => $value]);
        foreach (self::ROUTES as $path => [$operation]) {
            $this->emptyStatus($this->send($path, $this->body($operation)), 503);
        }
        self::assertNull(StoreSettings::current('delivery'));
    }

    public function test_store_guard_order_tls_identity_rate_limit_then_source_authority(): void
    {
        config(['products-entitlements.store.requests_per_minute' => 1]);
        $raw = $this->body('ReadStoreBalanceV1');
        foreach ([['SSL_PROTOCOL' => 'TLSv1.2'], ['SSL_CLIENT_VERIFY' => 'FAILED'],
            ['SSL_CLIENT_S_DN' => str_repeat('x', 129)], ['SSL_CLIENT_S_DN' => 'CN=wrong'], ['SSL_CLIENT_S_DN' => null]] as $server) {
            $this->emptyStatus($this->send('balance/read', $raw, $server), 401);
        }
        $wrongSource = str_replace(self::SOURCE, 'other.game', $raw);
        $this->emptyStatus($this->send('balance/read', $wrongSource), 401);
        $response = $this->send('balance/read', $raw);
        $this->emptyStatus($response, 429);
        self::assertGreaterThanOrEqual(1, (int) $response->headers->get('Retry-After'));
        $limiter = app(RateLimiter::class);
        self::assertSame(1, $limiter->attempts('products-entitlements-store:'.hash('sha256', self::PEER)));
        self::assertSame(0, $limiter->attempts('products-entitlements-store:'.self::PEER));
        // Disabling wins even when the peer has already exhausted its limiter.
        config(['products-entitlements.store.purchase_enabled' => false]);
        $this->emptyStatus($this->send('balance/read', $raw), 503);
    }

    public function test_each_store_route_enforces_transport_canonical_bytes_and_head(): void
    {
        foreach (self::ROUTES as $path => [$operation, $bound]) {
            $raw = $this->body($operation);
            foreach ([strval($bound + 1), '01', '-1', '10000000000', 'a'] as $length) {
                $this->emptyStatus($this->send($path, $raw, ['CONTENT_LENGTH' => $length]), 413);
            }
            $this->emptyStatus($this->send($path, str_repeat('a', $bound + 1)), 400);
            foreach (['', $raw."\n", ' '.$raw, str_replace('"contract_version":1', '"contract_version":2', $raw),
                str_replace('"contract_version":1,', '"contract_version":1,"contract_version":1,', $raw),
                str_replace('game.store.test', 'game\\u002estore.test', $raw),
                '{"operation":"'.$operation.'","contract_version":1,"source_authority":"'.self::SOURCE.'"}',
                str_replace($operation, 'UnknownOperation', $raw), '{"contract_version":1,"operation":"'.$operation.'"}',
                '{"contract_version":1,"operation":"'.$operation.'","source_authority":"'.self::SOURCE.'","nested":{"a":{"b":{}}}}'] as $invalid) {
                $this->emptyStatus($this->send($path, $invalid), 400);
            }
            foreach ([['HTTP_TRANSFER_ENCODING' => 'chunked'], ['HTTP_CONTENT_ENCODING' => 'gzip'], ['CONTENT_TYPE' => 'text/plain']] as $headers) {
                $this->emptyStatus($this->send($path, $raw, $headers), 400);
            }
            $this->emptyStatus($this->send($path, str_replace(self::SOURCE, 'another.game', $raw)), 401);
        }
    }

    public function test_restore_can_open_delivery_validation_with_purchases_off_but_handlers_remain_unavailable(): void
    {
        config(['products-entitlements.store.purchase_enabled' => false, 'products-entitlements.store.delivery_enabled' => false,
            'character-authority.restore.enabled' => true]);
        self::assertNotNull(StoreSettings::current('delivery'));
        $this->emptyStatus($this->send('operations/pending', $this->body('ListPendingStoreOperationsV1')), 503);
        $this->emptyStatus($this->send('operations/pending', $this->body('ListPendingStoreOperationsV1'), ['SSL_PROTOCOL' => 'TLSv1.2']), 401);
        $this->emptyStatus($this->send('balance/read', $this->body('ReadStoreBalanceV1'), ['SSL_PROTOCOL' => 'TLSv1.2']), 503);
    }

    public function test_large_declared_body_still_obeys_store_guard_order_and_other_routes_keep_global_limit(): void
    {
        $raw = $this->body('ReadStoreBalanceV1');
        $headers = ['CONTENT_LENGTH' => '9999999999', 'SSL_CLIENT_VERIFY' => 'FAILED'];
        config(['products-entitlements.store.purchase_enabled' => false]);
        $this->emptyStatus($this->send('balance/read', $raw, $headers), 503);
        config(['products-entitlements.store.purchase_enabled' => true]);
        $this->emptyStatus($this->send('balance/read', $raw, $headers), 401);
        foreach (['/internal/v1/game-auth/native-evidence', '/internal/v1/products-entitlements/store/unknown'] as $path) {
            $this->call('POST', $path, [], [], [],
                ['CONTENT_LENGTH' => '9999999999', 'CONTENT_TYPE' => 'application/json'], '{}')->assertStatus(413);
        }
    }

    private function body(string $operation): string
    {
        return '{"contract_version":1,"operation":"'.$operation.'","source_authority":"'.self::SOURCE.'"}';
    }

    /**
     * @param  array<string, mixed>  $server
     * @return TestResponse<Response>
     */
    private function send(string $path, string $raw, array $server = []): TestResponse
    {
        return $this->call('POST', '/internal/v1/products-entitlements/store/'.$path, [], [], [], array_replace([
            'CONTENT_TYPE' => 'application/json', 'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3', 'SSL_CLIENT_S_DN' => self::PEER,
        ], $server), $raw);
    }

    /** @param TestResponse<Response> $response */
    private function emptyStatus(TestResponse $response, int $status): void
    {
        $response->assertStatus($status)->assertContent('');
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
    }
}
