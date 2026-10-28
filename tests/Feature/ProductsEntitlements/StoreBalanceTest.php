<?php

namespace Tests\Feature\ProductsEntitlements;

use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;

final class StoreBalanceTest extends StorePurchaseTestCase
{
    use RefreshDatabase;

    public function test_balance_real_dispatch_is_read_only_coins_only_and_schema_valid(): void
    {
        $response = $this->send('balance/read', $this->readBody())->assertOk()
            ->assertJsonPath('balances.0.available', '0')->assertJsonCount(1, 'balances');
        self::assertSame(0, DB::table('wallet_accounts')->count());
        $bytes = $response->getContent();
        self::assertIsString($bytes);
        $this->schema('balance-response', $bytes);
        $this->fund();
        $this->send('purchases', $this->purchaseBody())->assertJsonPath('result', 'ACCEPTED');
        $this->send('balance/read', $this->readBody())->assertJsonPath('balances.0.available', '75')->assertJsonPath('balances.0.reserved', '0');
        $this->send('balance/read', $this->readBody(false, ['account_id' => (string) Str::uuid7()]))->assertStatus(404);
        self::assertSame(2, DB::table('wallet_ledger_entries')->count());
    }

    public function test_read_validation_and_shared_account_limiter(): void
    {
        $this->send('balance/read', substr($this->readBody(), 0, -1).',"extra":1}')->assertStatus(400);
        config(['products-entitlements.store.reads_per_account_per_minute' => 1]);
        $this->send('balance/read', $this->readBody())->assertOk();
        $this->send('history/read', $this->readBody(true))->assertStatus(429)->assertHeader('Retry-After');
        self::assertSame(0, DB::table('wallet_accounts')->count());
    }
}
