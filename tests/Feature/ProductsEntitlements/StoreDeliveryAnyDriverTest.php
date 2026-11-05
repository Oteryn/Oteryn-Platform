<?php

namespace Tests\Feature\ProductsEntitlements;

use App\Identity\Models\Identity;
use App\ProductsEntitlements\Store\Delivery\StoreDeliveryFeed;
use App\ProductsEntitlements\Store\Delivery\StoreDeliveryRefused;
use App\ProductsEntitlements\Store\Delivery\StoreReceiptPublisher;
use App\ProductsEntitlements\Store\Purchase\ReadStoreAccount;
use App\ProductsEntitlements\Store\StoreWire;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

/** Store delivery and account reads over an empty ledger; runs on every database driver. */
final class StoreDeliveryAnyDriverTest extends TestCase
{
    use RefreshDatabase;

    private const MISSING_ACCOUNT = '0192b6a0-4c00-7000-8000-0000000009a1';

    public function test_pending_and_reconcile_feeds_are_empty_without_operations(): void
    {
        $feed = $this->app->make(StoreDeliveryFeed::class);

        self::assertSame('{"contract_version":1,"pending":[]}', $feed->pending($this->canonical('pending-request.valid')));
        self::assertSame('{"contract_version":1,"operations":[]}', $feed->reconcile($this->canonical('reconcile-request.valid', [
            'after_source_revision' => '41', 'after_operation_id' => '0192b6a0-4c00-7000-8000-000000000701',
        ])));
    }

    public function test_feed_cursor_beyond_the_signed_sequence_returns_an_empty_page(): void
    {
        $page = $this->app->make(StoreDeliveryFeed::class)->pending($this->canonical('pending-request.valid', [
            'after_source_revision' => '18446744073709551615', 'after_operation_id' => '0192b6a0-4c00-7000-8000-000000000701',
        ]));

        self::assertSame('{"contract_version":1,"pending":[]}', $page);
    }

    public function test_reading_an_unknown_intent_is_refused_as_not_found(): void
    {
        try {
            $this->app->make(StoreDeliveryFeed::class)->read($this->canonical('read-request.valid'));
            self::fail('An unknown Store intent was read.');
        } catch (StoreDeliveryRefused $refused) {
            self::assertSame(404, $refused->status);
        }
    }

    public function test_a_receipt_without_its_intent_is_refused_without_writes(): void
    {
        try {
            $this->app->make(StoreReceiptPublisher::class)->publish($this->canonical('receipt.deliver-item.committed'));
            self::fail('A Store receipt without an intent was stored.');
        } catch (StoreDeliveryRefused $refused) {
            self::assertSame(404, $refused->status);
        }
        self::assertSame(0, DB::table('store_receipts')->count());
    }

    public function test_account_reads_answer_not_found_then_an_empty_balance_and_history(): void
    {
        $reads = new ReadStoreAccount;
        self::assertSame(404, $reads->read(['account_id' => self::MISSING_ACCOUNT], false)->getStatusCode());

        $identity = Identity::query()->create(['email' => 'store-any-driver@example.test',
            'password' => Hash::make('Store-any-driver-password-9!')]);
        $account = $identity->account_id;

        $balance = $reads->read(['account_id' => $account], false);
        self::assertSame(200, $balance->getStatusCode());
        $body = json_decode((string) $balance->getContent(), true, 8, JSON_THROW_ON_ERROR);
        self::assertIsArray($body);
        self::assertSame([['currency' => 'COINS', 'available' => '0', 'reserved' => '0']], $body['balances'] ?? null);

        $history = $reads->read(['account_id' => $account, 'before_entry_id' => '0', 'max_entries' => 2], true);
        self::assertSame(200, $history->getStatusCode());
        self::assertSame(StoreWire::encode(['contract_version' => 1, 'result' => 'OK', 'account_id' => $account,
            'entries' => [], 'next_before_entry_id' => '0'], 12288), $history->getContent());

        $cursor = $reads->read(['account_id' => $account, 'before_entry_id' => '41', 'max_entries' => 2], true);
        self::assertSame(200, $cursor->getStatusCode());
    }

    /** @param array<string, mixed> $changes */
    private function canonical(string $name, array $changes = []): string
    {
        $raw = file_get_contents(base_path('docs/contracts/fixtures/store-wire-v1/'.$name.'.json'));
        self::assertIsString($raw);
        /** @var array<string, mixed> $data */
        $data = json_decode($raw, true, 128, JSON_THROW_ON_ERROR);

        return StoreWire::encode(array_replace($data, $changes), 1024);
    }
}
