<?php

namespace Tests\Feature\ProductsEntitlements;

use App\Identity\Models\Identity;
use App\ProductsEntitlements\Store\StoreCatalogImporter;
use Closure;
use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\QueryException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Illuminate\Support\Str;
use Tests\TestCase;

final class StorePersistenceTest extends TestCase
{
    use RefreshDatabase;

    public function test_store_migrations_down_up_and_repeated_fresh_are_reversible_and_additive(): void
    {
        self::assertSame('pgsql', DB::getDriverName());
        $catalog = require database_path('migrations/2026_10_23_000100_create_store_catalog.php');
        $ledger = require database_path('migrations/2026_10_23_000200_create_store_ledger.php');
        self::assertInstanceOf(Migration::class, $catalog);
        self::assertInstanceOf(Migration::class, $ledger);
        self::assertTrue(method_exists($catalog, 'up'));
        self::assertTrue(method_exists($catalog, 'down'));
        self::assertTrue(method_exists($ledger, 'up'));
        self::assertTrue(method_exists($ledger, 'down'));
        for ($i = 0; $i < 2; $i++) {
            $ledger->down();
            $catalog->down();
            self::assertFalse(Schema::hasTable('store_operations'));
            self::assertFalse(Schema::hasTable('store_price_list_imports'));
            self::assertTrue(Schema::hasTable('wallet_accounts'));
            self::assertTrue(Schema::hasTable('identities'));
            $catalog->up();
            $ledger->up();
            self::assertSame(1, DB::table('store_active_catalog')->count());
            self::assertNull(DB::table('store_active_catalog')->value('catalog_revision'));
        }
    }

    public function test_store_request_and_wallet_idempotency_and_closed_sets_are_constrained(): void
    {
        [$request, $order] = $this->seedOrder();
        $this->refused(fn () => DB::table('store_purchase_requests')->insert($request));
        $this->refused(fn () => DB::table('store_purchase_requests')->insert(array_replace($request, ['purchase_request_id' => (string) Str::uuid7(), 'result' => 'UNAVAILABLE'])));
        $this->refused(fn () => DB::table('store_purchase_requests')->update(['answer_bytes' => '{}']));
        $this->refused(fn () => DB::table('store_orders')->insert(array_replace($order, ['order_id' => (string) Str::uuid7()])));
        $this->refused(fn () => DB::table('store_orders')->where('order_id', $order['order_id'])->update(['currency' => 'TRANSFERABLE_COINS']));
        $this->refused(fn () => DB::table('store_orders')->where('order_id', $order['order_id'])->update(['wallet_idempotency_key' => 'different']));
        $this->refused(fn () => DB::table('store_fulfilment_units')->update(['unit_kind' => 'OTHER']));
        $this->refused(fn () => DB::table('store_fulfilment_units')->update(['quantity' => 0]));
        $this->refused(fn () => DB::table('store_fulfilment_units')->update(['product_version' => '18446744073709551616']));
    }

    public function test_store_intents_exact_bytes_digest_monotone_sequence_and_one_deliver_per_unit(): void
    {
        $this->seedOrder();
        $operation = $this->intent();
        DB::table('store_operations')->insert($operation);
        self::assertSame($operation['intent_bytes'], DB::table('store_operations')->value('intent_bytes'));
        self::assertSame(hash('sha256', $operation['intent_bytes']), DB::table('store_operations')->value('binding_digest'));
        $this->refused(fn () => DB::table('store_operations')->insert($operation));
        $this->refused(fn () => DB::table('store_operations')->insert(array_replace($operation, ['operation_id' => (string) Str::uuid7()])));
        $this->refused(fn () => DB::table('store_operations')->update(['intent_bytes' => '{}']));
        $this->refused(fn () => DB::table('store_operations')->update(['binding_digest' => str_repeat('0', 64)]));
        $this->refused(fn () => DB::table('store_operations')->update(['operation_kind' => 'OTHER']));
        $this->refused(fn () => DB::table('store_operations')->delete());
        $first = DB::table('store_operations')->value('source_revision');
        self::assertIsInt($first);
        $claim = array_replace($operation, ['operation_id' => (string) Str::uuid7(), 'operation_kind' => 'CLAIM', 'delivery_operation_id' => $operation['operation_id']]);
        DB::table('store_operations')->insert($claim);
        self::assertGreaterThan($first, DB::table('store_operations')->max('source_revision'));
        self::assertSame(2, DB::table('store_operations')->count());
    }

    public function test_store_receipts_have_one_current_key_and_keep_immutable_superseded_history(): void
    {
        $this->seedOrder();
        $operation = $this->intent();
        DB::table('store_operations')->insert($operation);
        $receipt = ['issuer_authority' => $operation['issuer_authority'], 'operation_id' => $operation['operation_id'],
            'operation_kind' => 'DELIVER', 'binding_digest' => $operation['binding_digest'], 'outcome' => 'COMMITTED',
            'result_code' => 'STORE_OK', 'receipt_bytes' => '{"result":"STORE_OK"}',
            'receipt_digest' => hash('sha256', '{"result":"STORE_OK"}'), 'recovery_generation' => '1',
            'decided_at' => '1', 'received_at' => now()];
        $id = DB::table('store_receipts')->insertGetId($receipt);
        $this->refused(fn () => DB::table('store_receipts')->insert($receipt));
        $this->refused(fn () => DB::table('store_receipts')->update(['receipt_bytes' => '{}']));
        $this->refused(fn () => DB::table('store_receipts')->delete());
        $event = (string) Str::uuid7();
        DB::table('store_receipts')->where('id', $id)->update(['superseded_by_restore_event_id' => $event]);
        foreach ([['result_code' => 'OTHER'], ['result_code' => 'STORE_RECLAIMED'], ['recovery_generation' => '0'],
            ['receipt_digest' => str_repeat('f', 64)]] as $invalid) {
            $this->refused(fn () => DB::table('store_receipts')->insert(array_replace($receipt, $invalid)));
        }
        $current = DB::table('store_receipts')->insertGetId(array_replace($receipt, ['recovery_generation' => '2']));
        DB::table('store_reconcile_states')->insert(['restore_event_id' => $event, 'operation_id' => $operation['operation_id'],
            'superseded_receipt_id' => $id, 'restored_generation' => '1', 'new_recovery_generation' => '2', 'resolved_receipt_id' => $current]);
        $this->refused(fn () => DB::table('store_receipts')->where('id', $id)->update(['superseded_by_restore_event_id' => null]));
        self::assertSame(2, DB::table('store_receipts')->count());
        self::assertSame(1, DB::table('store_receipts')->whereNull('superseded_by_restore_event_id')->count());
        self::assertSame($receipt['receipt_bytes'], DB::table('store_receipts')->where('id', $id)->value('receipt_bytes'));
    }

    /** @return array{array<string, mixed>, array<string, mixed>} */
    private function seedOrder(): array
    {
        $identity = Identity::query()->create(['email' => 'store-fixture@example.test', 'password' => Hash::make('Store-fixture-password-9!')]);
        $raw = file_get_contents(base_path('docs/contracts/fixtures/store-price-list-v1/price-list.valid.json'));
        self::assertIsString($raw);
        $revision = app(StoreCatalogImporter::class)->import($raw);
        $request = ['purchase_request_id' => (string) Str::uuid7(), 'account_id' => $identity->account_id,
            'character_id' => (string) Str::uuid7(), 'world_id' => (string) Str::uuid7(), 'catalog_revision' => $revision,
            'offer_key' => 'test-supply', 'currency' => 'COINS', 'unit_price' => 25, 'quantity' => 1,
            'request_bytes' => '{"request":"fixture"}', 'request_digest' => hash('sha256', '{"request":"fixture"}'),
            'result' => 'ACCEPTED', 'answer_bytes' => '{"result":"ACCEPTED"}', 'decided_at' => now()];
        DB::table('store_purchase_requests')->insert($request);
        $order = ['order_id' => (string) Str::uuid7(), 'purchase_request_id' => $request['purchase_request_id'],
            'identity_id' => $identity->id, 'wallet_idempotency_key' => 'store-purchase:'.$request['purchase_request_id'],
            'currency' => 'COINS', 'total_price' => 25, 'created_at' => now()];
        DB::table('store_orders')->insert($order);
        $line = (string) Str::uuid7();
        DB::table('store_order_lines')->insert(['line_id' => $line, 'order_id' => $order['order_id'], 'catalog_revision' => $revision,
            'offer_key' => 'test-supply', 'quantity' => 1, 'unit_price' => 25]);
        DB::table('store_fulfilment_units')->insert(['entitlement_id' => (string) Str::uuid7(), 'line_id' => $line, 'unit_index' => 0,
            'account_id' => $identity->account_id, 'product_id' => 'oteryn.test_supply', 'product_version' => '1', 'unit_kind' => 'ITEM',
            'grant_key' => 'test-supply', 'grant_revision' => '1', 'quantity' => 5, 'state' => 'DELIVERY_PENDING', 'created_at' => now()]);
        self::assertSame($identity->id, Identity::query()->where('account_id', $identity->account_id)->value('id'));

        return [$request, $order];
    }

    /** @return array{operation_id:string,issuer_authority:string,operation_kind:string,entitlement_id:string,intent_bytes:string,binding_digest:string,state:string,created_at:mixed} */
    private function intent(): array
    {
        $entitlement = DB::table('store_fulfilment_units')->value('entitlement_id');
        self::assertIsString($entitlement);

        return ['operation_id' => (string) Str::uuid7(), 'issuer_authority' => 'OTERYN_PLATFORM_STORE_AUTHORITY',
            'operation_kind' => 'DELIVER', 'entitlement_id' => $entitlement,
            'intent_bytes' => '{"intent":"fixture"}', 'binding_digest' => hash('sha256', '{"intent":"fixture"}'),
            'state' => 'PENDING', 'created_at' => now()];
    }

    /** @param Closure(): mixed $mutation */
    private function refused(Closure $mutation): void
    {
        try {
            DB::transaction($mutation);
            self::fail('PostgreSQL accepted invalid Store state.');
        } catch (QueryException $exception) {
            self::assertNotSame('', $exception->getCode());
        }
    }
}
