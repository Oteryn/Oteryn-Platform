<?php

namespace Tests\Feature\ProductsEntitlements;

use App\ProductsEntitlements\Store\Models\StoreFulfilmentUnit;
use App\ProductsEntitlements\Store\Models\StoreOperation;
use App\ProductsEntitlements\Store\Models\StoreReceipt;
use App\ProductsEntitlements\Store\StoreWire;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Str;
use Tests\TestCase;

final class StoreDeliveryFeedTest extends TestCase
{
    use RefreshDatabase;
    use StoreDeliveryFixtures;

    protected function setUp(): void
    {
        parent::setUp();
        $this->configureDelivery();
    }

    public function test_store_pending_paging_is_stable_and_never_expires_or_writes(): void
    {
        $this->seedDelivery();
        $request = $this->pageRequest(fields: ['max_entries' => 2]);
        $first = $this->deliveryRequest('pending', $request)->assertOk();
        $this->assertWireSchema($this->responseBytes($first), 'pending-response');
        $first->assertJsonPath('pending.0.source_revision', '101')->assertJsonPath('pending.1.source_revision', '102');
        $later = $this->fixture('intent.flag');
        $later['operation_id'] = (string) Str::uuid7();
        $later['source_revision'] = '108';
        $this->seedIntent($later);
        $this->deliveryRequest('pending', $request)->assertContent($this->responseBytes($first));
        $request = $this->pageRequest(fields: ['after_source_revision' => '102',
            'after_operation_id' => $this->fixture('intent.deliver-cosmetic')['operation_id']]);
        $this->deliveryRequest('pending', $request)->assertOk()->assertJsonCount(6, 'pending')->assertJsonPath('pending.0.source_revision', '103');
        $beforeTime = $this->responseBytes($this->deliveryRequest('pending', $request));
        $this->travel(100)->years();
        $this->deliveryRequest('pending', $request)->assertContent($beforeTime);
        self::assertSame(8, StoreOperation::query()->where('state', 'PENDING')->count());
        self::assertSame(3, StoreFulfilmentUnit::query()->where('state', 'DELIVERY_PENDING')->count());
        self::assertSame(0, StoreReceipt::query()->count());
        $end = $this->pageRequest(fields: ['after_source_revision' => '108', 'after_operation_id' => $later['operation_id']]);
        $this->deliveryRequest('pending', $end)->assertOk()->assertJsonCount(0, 'pending');
    }

    public function test_store_read_returns_exact_stored_bytes_before_and_after_a_receipt(): void
    {
        $this->seedDelivery();
        $intent = $this->fixture('intent.deliver-item');
        $key = array_replace($this->fixture('read-request.valid'), ['operation_kind' => 'DELIVER', 'operation_id' => $intent['operation_id']]);
        $bytes = StoreWire::encode($intent, 4096);
        $this->deliveryRequest('read', $key)->assertOk()->assertContent($bytes);
        $this->assertWireSchema($bytes, 'intent');
        $this->deliveryRequest('receipt', $this->fixture('receipt.deliver-item.committed'))->assertOk();
        $this->deliveryRequest('read', $key)->assertOk()->assertContent($bytes);
        $this->deliveryRequest('read', array_replace($key, ['operation_kind' => 'CLAIM']))->assertNotFound()->assertContent('');
        $this->deliveryRequest('read', array_replace($key, ['operation_id' => (string) Str::uuid7()]))->assertNotFound();
    }

    public function test_store_delivery_closed_members_order_cursor_and_bounds_are_enforced(): void
    {
        foreach (['pending', 'reconcile'] as $path) {
            $request = $this->pageRequest($path === 'reconcile');
            foreach ([['max_entries' => 0], ['max_entries' => 33], ['max_entries' => '1'],
                ['after_source_revision' => '18446744073709551616'], ['after_source_revision' => '01'],
                ['after_operation_id' => (string) Str::uuid7()], ['extra' => 1], ['after_source_revision' => '1']] as $bad) {
                $this->deliveryRequest($path, array_replace($request, $bad))->assertStatus(400)->assertContent('');
            }
            $reordered = $request;
            unset($reordered['after_source_revision']);
            $reordered['after_source_revision'] = '0';
            $this->deliveryRequest($path, $reordered)->assertStatus(400);
            $huge = array_replace($request, ['after_source_revision' => '18446744073709551615', 'after_operation_id' => (string) Str::uuid7()]);
            $this->deliveryRequest($path, $huge)->assertOk()->assertJsonCount(0, $path === 'pending' ? 'pending' : 'operations');
            $this->deliveryRequest($path, $request, ['CONTENT_LENGTH' => '513'])->assertStatus(413);
        }
    }

    public function test_store_delivery_guard_flags_and_dependency_failure_preserve_empty_no_store_responses(): void
    {
        foreach (['pending' => $this->pageRequest(), 'reconcile' => $this->pageRequest(true),
            'read' => $this->fixture('read-request.valid'), 'receipt' => $this->fixture('receipt.deliver-item.committed')] as $path => $body) {
            config(['products-entitlements.store.delivery_enabled' => false]);
            $this->deliveryRequest($path, $body, ['SSL_CLIENT_VERIFY' => 'NONE'])->assertStatus(503)->assertContent('');
            config(['products-entitlements.store.delivery_enabled' => true]);
            $this->deliveryRequest($path, $body, ['SSL_CLIENT_VERIFY' => 'NONE'])->assertUnauthorized();
            $this->deliveryRequest($path, array_replace($body, ['source_authority' => 'other.source']))->assertUnauthorized();
        }
        config(['products-entitlements.store.delivery_enabled' => false, 'character-authority.restore.enabled' => true]);
        $response = $this->deliveryRequest('pending', $this->pageRequest())->assertOk();
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        self::assertSame(0, StoreReceipt::query()->count());
    }
}
