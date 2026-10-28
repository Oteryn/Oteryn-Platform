<?php

namespace Tests\Feature\ProductsEntitlements;

use App\CharacterAuthority\Restore\GameRestoreReconciler;
use App\ProductsEntitlements\Store\Models\StoreOperation;
use App\ProductsEntitlements\Store\Models\StoreReceipt;
use App\ProductsEntitlements\Store\Models\StoreReconcileState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use Tests\TestCase;

final class StoreReconcileFeedTest extends TestCase
{
    use RefreshDatabase;
    use StoreDeliveryFixtures;

    protected function setUp(): void
    {
        parent::setUp();
        $this->configureDelivery();
    }

    public function test_store_reconcile_lists_all_states_with_current_receipt_and_bounded_stable_pages(): void
    {
        $this->seedDelivery();
        $receipt = $this->fixture('receipt.deliver-item.committed');
        $this->deliveryRequest('receipt', $receipt)->assertOk();
        $first = $this->deliveryRequest('reconcile', $this->pageRequest(true))->assertOk();
        $first->assertJsonCount(7, 'operations')->assertJsonPath('operations.0.state', 'PENDING')
            ->assertJsonPath('operations.0.receipt_digest', null)->assertJsonPath('operations.2.state', 'COMMITTED')
            ->assertJsonPath('operations.2.recovery_generation', '7')
            ->assertJsonPath('operations.2.receipt_digest', StoreReceipt::query()->value('receipt_digest'));
        $this->assertWireSchema($this->responseBytes($first), 'reconcile-response');
        $flag = $this->fixture('intent.flag');
        for ($i = 0; $i < 35; $i++) {
            $this->seedIntent(array_replace($flag, ['source_revision' => (string) (200 + $i), 'operation_id' => (string) Str::uuid7()]));
        }
        $request = $this->pageRequest(true, ['max_entries' => 32]);
        $page = $this->deliveryRequest('reconcile', $request)->assertOk()->assertJsonCount(32, 'operations');
        self::assertLessThanOrEqual(12288, strlen($this->responseBytes($page)));
        $this->assertWireSchema($this->responseBytes($page), 'reconcile-response');
        $cursor = $page->json('operations.31');
        self::assertIsArray($cursor);
        $next = $this->pageRequest(true, ['after_source_revision' => $cursor['source_revision'], 'after_operation_id' => $cursor['operation_id']]);
        $this->deliveryRequest('reconcile', $next)->assertOk()->assertJsonCount(10, 'operations');
        $this->deliveryRequest('reconcile', $request)->assertContent($this->responseBytes($page));
        $this->deliveryRequest('pending', $this->pageRequest(fields: ['max_entries' => 32]))->assertOk()->assertJsonCount(32, 'pending');
        self::assertSame(1, StoreReceipt::query()->count());
        self::assertSame(42, StoreOperation::query()->count());
        self::assertSame(0, StoreReconcileState::query()->count());
    }

    public function test_store_reopened_history_is_read_only_and_only_a_new_generation_can_resolve_it(): void
    {
        $this->seedDelivery();
        $receipt = $this->fixture('receipt.deliver-item.committed');
        $this->deliveryRequest('receipt', $receipt)->assertOk();
        $stored = StoreReceipt::query()->firstOrFail();
        $event = (string) Str::uuid7();
        $stored->update(['superseded_by_restore_event_id' => $event]);
        StoreOperation::query()->where('operation_id', $receipt['operation_id'])->update(['state' => 'REOPENED']);
        $history = StoreReconcileState::query()->create(['restore_event_id' => $event, 'operation_id' => $receipt['operation_id'],
            'superseded_receipt_id' => $stored->getKey(), 'restored_generation' => '7', 'new_recovery_generation' => '8']);
        $before = $history->refresh()->toArray();
        $this->deliveryRequest('reconcile', $this->pageRequest(true))->assertOk()
            ->assertJsonPath('operations.2.state', 'REOPENED')->assertJsonPath('operations.2.receipt_digest', null)
            ->assertJsonPath('operations.2.recovery_generation', null);
        $this->deliveryRequest('pending', $this->pageRequest())->assertOk()->assertJsonCount(7, 'pending');
        self::assertSame($before, $history->fresh()?->toArray());
        $this->deliveryRequest('receipt', $receipt)->assertStatus(409);
        $new = array_replace($receipt, ['recovery_generation' => '8']);
        $this->deliveryRequest('receipt', $new)->assertOk();
        $this->deliveryRequest('receipt', $new)->assertOk();
        self::assertSame(2, StoreReceipt::query()->count());
        self::assertSame($event, $stored->fresh()?->superseded_by_restore_event_id);
        self::assertNotNull($history->refresh()->resolved_receipt_id);
        $this->deliveryRequest('reconcile', $this->pageRequest(true))->assertOk()
            ->assertJsonPath('operations.2.state', 'COMMITTED')->assertJsonPath('operations.2.recovery_generation', '8');
    }

    public function test_store_latest_global_restore_fences_new_receipts_but_accepts_a_still_current_exact_republication(): void
    {
        $this->seedDelivery();
        $receipt = $this->fixture('receipt.deliver-item.committed');
        $this->deliveryRequest('receipt', $receipt)->assertOk();
        DB::table(GameRestoreReconciler::NOTICES)->insert(['restore_event_id' => (string) Str::uuid7(),
            'source_authority' => 'oteryn-game/restore', 'restored_generation' => str_pad('7', 20, '0', STR_PAD_LEFT),
            'new_recovery_generation' => str_pad('8', 20, '0', STR_PAD_LEFT), 'notice_json' => '{}',
            'notice_sha256' => hash('sha256', '{}'), 'reopened_count' => 0, 'received_at' => now()]);
        $this->deliveryRequest('receipt', $receipt)->assertOk();
        $premium = $this->fixture('receipt.deliver-premium-time.committed');
        $this->deliveryRequest('receipt', $premium)->assertStatus(409)->assertContent('');
        self::assertSame(1, StoreReceipt::query()->count());
        $this->deliveryRequest('receipt', array_replace($premium, ['recovery_generation' => '8']))->assertOk();
        self::assertSame(2, StoreReceipt::query()->count());
    }
}
