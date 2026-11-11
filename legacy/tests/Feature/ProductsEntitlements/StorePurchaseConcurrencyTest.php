<?php

namespace Tests\Feature\ProductsEntitlements;

use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Tests\Support\Concurrency\ForkedWorkers;

final class StorePurchaseConcurrencyTest extends StorePurchaseTestCase
{
    use DatabaseMigrations;

    public function test_concurrent_purchase_dispatch_never_overspends_one_wallet(): void
    {
        self::assertTrue(function_exists('pcntl_fork'), 'Store concurrency qualification requires pcntl.');
        $this->fund(25);
        $requests = [$this->purchaseBody(), $this->purchaseBody()];
        $results = ForkedWorkers::run(2, 'pgsql', function (int $worker) use ($requests): array {
            $response = $this->send('purchases', $requests[$worker]);

            return ['status' => $response->status(), 'result' => $response->json('result')];
        });
        self::assertSame([200, 200], array_column($results, 'status'));
        $outcomes = array_column($results, 'result');
        sort($outcomes);
        self::assertSame(['ACCEPTED', 'INSUFFICIENT_BALANCE'], $outcomes);
        self::assertSame(0, DB::table('wallet_accounts')->value('available_balance'));
        self::assertSame(1, DB::table('wallet_ledger_entries')->where('operation_type', 'store_purchase')->count());
        self::assertSame(1, DB::table('store_orders')->count());
        self::assertSame(2, DB::table('store_purchase_requests')->count());
    }

    public function test_concurrent_first_attempts_return_identical_bytes_and_debit_once(): void
    {
        self::assertTrue(function_exists('pcntl_fork'));
        $this->fund(100);
        $raw = $this->purchaseBody();
        $results = ForkedWorkers::run(2, 'pgsql', function (int $worker) use ($raw): array {
            $response = $this->send('purchases', $raw);

            return ['status' => $response->status(), 'bytes' => $response->getContent()];
        });
        self::assertSame([200, 200], array_column($results, 'status'));
        self::assertSame($results[0]['bytes'], $results[1]['bytes']);
        self::assertSame(75, DB::table('wallet_accounts')->value('available_balance'));
        self::assertSame(1, DB::table('store_orders')->count());
        self::assertSame(1, DB::table('store_purchase_requests')->count());
    }

    public function test_wallet_lock_timeout_is_unavailable_without_any_partial_purchase(): void
    {
        $this->fund();
        config(['database.connections.store_test_lock' => config('database.connections.pgsql')]);
        $holder = DB::connection('store_test_lock');
        $holder->beginTransaction();
        $holder->table('wallet_accounts')->where('identity_id', $this->buyer->id)->lockForUpdate()->first();
        $raw = $this->purchaseBody();
        try {
            $this->send('purchases', $raw)->assertStatus(503)->assertContent('');
            self::assertSame(0, DB::table('store_purchase_requests')->count());
            self::assertSame(0, DB::table('store_orders')->count());
            self::assertSame(0, DB::table('store_operations')->count());
            self::assertSame(100, DB::table('wallet_accounts')->value('available_balance'));
        } finally {
            $holder->rollBack();
            DB::purge('store_test_lock');
        }
        $this->send('purchases', $raw)->assertOk()->assertJsonPath('result', 'ACCEPTED');
    }
}
