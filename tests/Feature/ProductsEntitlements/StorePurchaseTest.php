<?php

namespace Tests\Feature\ProductsEntitlements;

use App\ProductsEntitlements\Store\StoreWire;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Log;
use Illuminate\Support\Str;
use PHPUnit\Framework\Attributes\DataProvider;

final class StorePurchaseTest extends StorePurchaseTestCase
{
    use RefreshDatabase;

    public function test_real_dispatch_accepts_atomic_order_and_identical_retry_debits_once(): void
    {
        $this->fund();
        $raw = $this->purchaseBody(['quantity' => 2]);
        $this->schema('purchase-request', $raw, true);
        $response = $this->send('purchases', $raw)->assertOk()->assertJsonPath('result', 'ACCEPTED')
            ->assertJsonPath('total_price', '50')->assertJsonPath('available_after', '50');
        $bytes = $response->getContent();
        self::assertIsString($bytes);
        $this->schema('purchase-answer', $bytes);
        $this->travel(5)->minutes();
        config(['products-entitlements.store.enabled_unit_kinds' => []]);
        $this->send('purchases', $raw)->assertOk()->assertContent($bytes);
        self::assertSame(1, DB::table('wallet_ledger_entries')->where('operation_type', 'store_purchase')->count());
        self::assertSame(1, DB::table('store_orders')->count());
        self::assertSame(1, DB::table('store_order_lines')->count());
        self::assertSame(1, DB::table('store_fulfilment_units')->count());
        self::assertSame(10, DB::table('store_fulfilment_units')->value('quantity'));
        $intent = DB::table('store_operations')->first();
        self::assertNotNull($intent);
        self::assertIsString($intent->intent_bytes);
        self::assertSame(hash('sha256', $intent->intent_bytes), $intent->binding_digest);
        self::assertSame('PENDING', $intent->state);
        $this->schema('intent', $intent->intent_bytes, true);
    }

    public function test_conflicting_reuse_alarms_and_writes_nothing(): void
    {
        $this->fund();
        $raw = $this->purchaseBody();
        $bytes = $this->send('purchases', $raw)->assertOk()->getContent();
        $this->withoutExceptionHandling();
        Log::shouldReceive('info')->zeroOrMoreTimes();
        Log::shouldReceive('warning')->once()->with('products_entitlements.store_purchase_request_conflict');
        $this->send('purchases', str_replace('"quantity":1', '"quantity":2', $raw))->assertStatus(409)->assertContent('');
        self::assertSame($bytes, DB::table('store_purchase_requests')->value('answer_bytes'));
        self::assertSame(75, DB::table('wallet_accounts')->value('available_balance'));
        self::assertSame(1, DB::table('store_orders')->count());
    }

    /** @return iterable<string,array{string}> */
    public static function refusals(): iterable
    {
        foreach (['ACCOUNT_RESTRICTED', 'STALE_QUOTE', 'OFFER_UNAVAILABLE', 'PURCHASE_DISABLED', 'INSUFFICIENT_BALANCE'] as $result) {
            yield $result => [$result];
        }
    }

    #[DataProvider('refusals')]
    public function test_terminal_refusal_never_becomes_purchase_after_topup_flags_or_import(string $result): void
    {
        $changes = match ($result) {
            'ACCOUNT_RESTRICTED' => ['account_id' => (string) Str::uuid7()],
            'STALE_QUOTE' => ['unit_price' => '26'],
            'OFFER_UNAVAILABLE' => ['quantity' => 11],
            default => [],
        };
        if ($result === 'PURCHASE_DISABLED') {
            config(['products-entitlements.store.enabled_unit_kinds' => []]);
        }
        $raw = $this->purchaseBody($changes);
        $response = $this->send('purchases', $raw)->assertOk()->assertJsonPath('result', $result);
        $bytes = $response->getContent();
        self::assertIsString($bytes);
        $this->schema('purchase-answer', $bytes);
        self::assertSame(0, DB::table('wallet_accounts')->count());
        self::assertSame(0, DB::table('wallet_ledger_entries')->count());
        $this->fund();
        config(['products-entitlements.store.enabled_unit_kinds' => ['ITEM']]);
        $fixture = file_get_contents(base_path('docs/contracts/fixtures/store-price-list-v1/price-list.valid.json'));
        self::assertIsString($fixture);
        $this->catalog(str_replace('"25"', '"26"', $fixture));
        $this->send('purchases', $raw)->assertOk()->assertContent($bytes);
        self::assertSame(0, DB::table('store_orders')->count());
        self::assertSame(1, DB::table('store_purchase_requests')->count());
    }

    public function test_database_failure_after_debit_rolls_back_every_effect_and_can_retry(): void
    {
        $this->fund();
        DB::unprepared("CREATE FUNCTION store_test_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected'; END; $$; CREATE TRIGGER store_test_fail BEFORE INSERT ON store_orders FOR EACH ROW EXECUTE FUNCTION store_test_fail();");
        $raw = $this->purchaseBody();
        $this->send('purchases', $raw)->assertStatus(503)->assertContent('');
        self::assertSame(100, DB::table('wallet_accounts')->value('available_balance'));
        self::assertSame(0, DB::table('wallet_ledger_entries')->where('operation_type', 'store_purchase')->count());
        foreach (['store_purchase_requests', 'store_orders', 'store_order_lines', 'store_fulfilment_units', 'store_operations'] as $table) {
            self::assertSame(0, DB::table($table)->count());
        }
        DB::unprepared('DROP TRIGGER store_test_fail ON store_orders; DROP FUNCTION store_test_fail();');
        $this->send('purchases', $raw)->assertOk()->assertJsonPath('result', 'ACCEPTED');
    }

    public function test_surface_off_and_missing_catalog_are_unavailable_without_deciding_request(): void
    {
        $raw = $this->purchaseBody();
        config(['products-entitlements.store.purchase_enabled' => false]);
        $this->send('purchases', $raw)->assertStatus(503);
        config(['products-entitlements.store.purchase_enabled' => true]);
        DB::table('store_active_catalog')->update(['catalog_revision' => null, 'activated_at' => null]);
        $this->send('purchases', $raw)->assertStatus(503);
        self::assertSame(0, DB::table('store_purchase_requests')->count());
    }

    public function test_account_lock_quote_offer_overflow_and_transferable_coin_gates(): void
    {
        $this->buyer->forceFill(['disabled_at' => now()])->save();
        $this->send('purchases', $this->purchaseBody())->assertJsonPath('result', 'ACCOUNT_RESTRICTED');
        $this->buyer->forceFill(['disabled_at' => null])->save();
        foreach ([['catalog_revision' => str_repeat('f', 64)], ['unit_price' => '24'], ['currency' => 'TRANSFERABLE_COINS']] as $change) {
            $this->send('purchases', $this->purchaseBody($change))->assertJsonPath('result', 'STALE_QUOTE');
        }
        $this->send('purchases', $this->purchaseBody(['offer_key' => 'missing']))->assertJsonPath('result', 'OFFER_UNAVAILABLE');
        $fixture = file_get_contents(base_path('docs/contracts/fixtures/store-price-list-v1/price-list.valid.json'));
        self::assertIsString($fixture);
        foreach ([str_replace('"COINS"', '"TRANSFERABLE_COINS"', $fixture), str_replace('"25"', '"9223372036854775807"', $fixture), str_replace('"purchasable":true', '"purchasable":false', $fixture)] as $raw) {
            $revision = $this->catalog($raw);
            $data = json_decode($raw, true, 8, JSON_THROW_ON_ERROR);
            self::assertIsArray($data);
            self::assertIsArray($data['offers']);
            self::assertIsArray($data['offers'][0]);
            $this->send('purchases', $this->purchaseBody(['catalog_revision' => $revision, 'currency' => $data['offers'][0]['currency'],
                'unit_price' => $data['offers'][0]['unit_price'], 'quantity' => 2]))->assertJsonPath('result', 'OFFER_UNAVAILABLE');
        }
        self::assertSame(0, DB::table('wallet_accounts')->count());
    }

    public function test_full_body_validation_is_closed_ordered_and_bounded_and_account_rate_limit_is_nonterminal(): void
    {
        $raw = $this->purchaseBody();
        foreach ([str_replace('"quantity":1', '"quantity":101', $raw), str_replace('"quantity":1', '"quantity":"1"', $raw),
            str_replace('"unit_price":"25"', '"unit_price":"9223372036854775808"', $raw), substr($raw, 0, -1).',"extra":1}',
            str_replace($this->revision, '1', $raw), str_replace('"quantity":1', '"quantity":0', $raw)] as $invalid) {
            $this->send('purchases', $invalid)->assertStatus(400);
        }
        config(['products-entitlements.store.purchases_per_account_per_minute' => 1]);
        $this->send('purchases', $raw)->assertOk();
        $this->send('purchases', $this->purchaseBody())->assertStatus(429)->assertHeader('Retry-After');
        self::assertSame(1, DB::table('store_purchase_requests')->count());
    }

    public function test_every_kind_gets_an_intent_and_pending_capacity_is_retryable(): void
    {
        $fixture = file_get_contents(base_path('docs/contracts/fixtures/store-price-list-v1/price-list.valid.json'));
        self::assertIsString($fixture);
        $units = [];
        foreach (['PREMIUM_TIME', 'COSMETIC', 'ITEM'] as $index => $kind) {
            $units[] = ['product_id' => 'oteryn.test_unit', 'product_version' => (string) (3 - $index),
                'unit_kind' => $kind, 'grant_key' => 'test-unit', 'grant_revision' => '1', 'quantity' => 1];
        }
        $catalog = ['contract_version' => 1, 'offers' => [['offer_key' => 'test-supply', 'currency' => 'COINS',
            'unit_price' => '25', 'max_quantity' => 10, 'purchasable' => true, 'units' => $units]]];
        $rawCatalog = StoreWire::encode($catalog, 4096);
        $this->revision = $this->catalog($rawCatalog);
        $this->fund();
        $raw = $this->purchaseBody();
        config(['products-entitlements.store.max_pending_operations' => 2]);
        $this->send('purchases', $raw)->assertStatus(503);
        self::assertSame(0, DB::table('store_purchase_requests')->count());
        config(['products-entitlements.store.max_pending_operations' => 3]);
        $this->send('purchases', $raw)->assertOk()->assertJsonPath('result', 'ACCEPTED')->assertJsonCount(3, 'units');
        self::assertSame(3, DB::table('store_operations')->where('operation_kind', 'DELIVER')->count());
        self::assertSame(['1', '2', '3'], DB::table('store_fulfilment_units')->orderBy('unit_index')->pluck('product_version')->all());
        $this->send('purchases', $raw)->assertOk()->assertJsonPath('result', 'ACCEPTED');
        $this->send('purchases', $this->purchaseBody())->assertStatus(503);
        self::assertSame(1, DB::table('store_purchase_requests')->count());
        foreach (DB::table('store_operations')->limit(8)->get() as $operation) {
            self::assertIsString($operation->intent_bytes);
            $this->schema('intent', $operation->intent_bytes, true);
        }
    }

    public function test_restricted_account_refusal_survives_unlock_and_unit_overflow_is_terminal(): void
    {
        $this->fund();
        $this->buyer->forceFill(['disabled_at' => now()])->save();
        $raw = $this->purchaseBody();
        $bytes = $this->send('purchases', $raw)->assertJsonPath('result', 'ACCOUNT_RESTRICTED')->getContent();
        self::assertIsString($bytes);
        $this->buyer->forceFill(['disabled_at' => null])->save();
        $this->send('purchases', $raw)->assertContent($bytes);
        $fixture = file_get_contents(base_path('docs/contracts/fixtures/store-price-list-v1/price-list.valid.json'));
        self::assertIsString($fixture);
        $this->revision = $this->catalog(str_replace('"quantity":5', '"quantity":65535', $fixture));
        $this->send('purchases', $this->purchaseBody(['quantity' => 2]))->assertOk()->assertJsonPath('result', 'OFFER_UNAVAILABLE');
        self::assertSame(100, DB::table('wallet_accounts')->value('available_balance'));
        self::assertSame(0, DB::table('store_orders')->count());
    }
}
