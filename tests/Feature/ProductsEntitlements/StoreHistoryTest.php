<?php

namespace Tests\Feature\ProductsEntitlements;

use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;

final class StoreHistoryTest extends StorePurchaseTestCase
{
    use RefreshDatabase;

    public function test_history_real_dispatch_pages_stably_and_exposes_only_own_safe_fields(): void
    {
        $this->fund();
        $this->send('purchases', $this->purchaseBody())->assertJsonPath('result', 'ACCEPTED');
        $this->fund();
        $other = Identity::query()->create(['email' => 'other-store@example.test', 'password' => 'unused']);
        $this->fund(123456, $other);
        $page = $this->send('history/read', $this->readBody(true))->assertOk()->assertJsonCount(2, 'entries')
            ->assertJsonPath('entries.0.kind', 'OPERATOR_ADJUSTMENT')->assertJsonPath('entries.1.kind', 'STORE_PURCHASE')
            ->assertJsonPath('entries.1.offer_key', 'test-supply')->assertJsonPath('entries.1.quantity', 1);
        $bytes = $page->getContent();
        self::assertIsString($bytes);
        $this->schema('history-response', $bytes);
        self::assertStringNotContainsString('metadata', $bytes);
        self::assertStringNotContainsString('idempotency', $bytes);
        self::assertStringNotContainsString('123456', $bytes);
        $cursor = $page->json('next_before_entry_id');
        self::assertIsString($cursor);
        self::assertNotSame('0', $cursor);
        $request = $this->readBody(true, ['before_entry_id' => $cursor]);
        $older = $this->send('history/read', $request)->assertOk()->assertJsonCount(1, 'entries')
            ->assertJsonPath('next_before_entry_id', '0')->getContent();
        self::assertIsString($older);
        $this->fund(50);
        $this->send('purchases', $this->purchaseBody())->assertJsonPath('result', 'ACCEPTED');
        $this->send('history/read', $request)->assertContent($older);
        $this->send('history/read', $this->readBody(true, ['account_id' => $other->account_id]))
            ->assertJsonCount(1, 'entries')->assertJsonPath('entries.0.available_delta', '123456');
    }

    public function test_empty_unknown_unmapped_and_uint64_history_cursor_bounds(): void
    {
        $this->send('history/read', $this->readBody(true))->assertOk()->assertJsonCount(0, 'entries')->assertJsonPath('next_before_entry_id', '0');
        $this->send('history/read', $this->readBody(true, ['account_id' => (string) Str::uuid7()]))->assertStatus(404);
        $this->fund();
        DB::table('wallet_ledger_entries')->update(['operation_type' => 'future_unknown', 'metadata' => '{"provider":"do-not-expose"}']);
        $this->send('history/read', $this->readBody(true, ['before_entry_id' => '18446744073709551615']))->assertOk()->assertJsonPath('entries.0.kind', 'OTHER');
        foreach ([['max_entries' => 33], ['max_entries' => 0], ['before_entry_id' => '01'], ['before_entry_id' => '18446744073709551616']] as $invalid) {
            $this->send('history/read', $this->readBody(true, $invalid))->assertStatus(400);
        }
    }
}
