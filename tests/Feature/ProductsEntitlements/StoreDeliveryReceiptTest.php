<?php

namespace Tests\Feature\ProductsEntitlements;

use App\ProductsEntitlements\Store\Models\StoreFulfilmentUnit;
use App\ProductsEntitlements\Store\Models\StoreOperation;
use App\ProductsEntitlements\Store\Models\StoreReceipt;
use App\ProductsEntitlements\Store\StoreWire;
use Illuminate\Database\QueryException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Log;
use Illuminate\Support\Str;
use PDOException;
use Tests\TestCase;

final class StoreDeliveryReceiptTest extends TestCase
{
    use RefreshDatabase;
    use StoreDeliveryFixtures;

    protected function setUp(): void
    {
        parent::setUp();
        $this->configureDelivery();
    }

    public function test_store_first_receipt_wins_identical_republication_is_accepted_and_conflict_alarms_without_writes(): void
    {
        $this->seedDelivery();
        $receipt = $this->fixture('receipt.deliver-item.committed');
        $answer = $this->deliveryRequest('receipt', $receipt)->assertOk()->assertContent('{"contract_version":1,"result":"accepted"}');
        $this->assertWireSchema($this->responseBytes($answer), 'receipt-answer');
        $stored = StoreReceipt::query()->firstOrFail()->toArray();
        $this->deliveryRequest('receipt', $receipt)->assertOk()->assertContent($this->responseBytes($answer));
        $log = Log::spy();
        $this->deliveryRequest('receipt', array_replace($receipt, ['decided_at' => '1792756813']))->assertStatus(409)->assertContent('');
        $log->shouldHaveReceived('error')->once()->with('products_entitlements.store_receipt_conflict', [
            'operation_id' => $receipt['operation_id'], 'operation_kind' => 'DELIVER', 'result_code' => 'STORE_OK']);
        self::assertSame($stored, StoreReceipt::query()->firstOrFail()->toArray());
        self::assertSame(1, StoreReceipt::query()->count());
        self::assertSame(StoreWire::encode($receipt, 1024), StoreReceipt::query()->firstOrFail()->receipt_bytes);
        self::assertSame('COMMITTED', StoreOperation::query()->where('operation_id', $receipt['operation_id'])->value('state'));
        self::assertSame('DELIVERED', StoreFulfilmentUnit::query()->whereKey($receipt['entitlement_id'])->value('state'));
        self::assertSame(7, StoreOperation::query()->count()); // Delivery never mints another operation id.
        $this->deliveryRequest('pending', $this->pageRequest())->assertOk()->assertJsonCount(6, 'pending');
    }

    public function test_store_rejection_is_terminal_and_moves_unit_to_support_review_only_on_receipt(): void
    {
        $this->seedDelivery();
        $receipt = $this->fixture('receipt.deliver-item.committed');
        unset($receipt['delivered_quantity']);
        $receipt = array_replace($receipt, ['outcome' => 'REJECTED', 'result_code' => 'STORE_GRANT_UNKNOWN']);
        self::assertSame('DELIVERY_PENDING', StoreFulfilmentUnit::query()->whereKey($receipt['entitlement_id'])->value('state'));
        $this->deliveryRequest('receipt', $receipt)->assertOk();
        $this->deliveryRequest('receipt', $receipt)->assertOk();
        self::assertSame('REJECTED', StoreOperation::query()->where('operation_id', $receipt['operation_id'])->value('state'));
        self::assertSame('MANUAL_REVIEW', StoreFulfilmentUnit::query()->whereKey($receipt['entitlement_id'])->value('state'));
        self::assertSame(1, StoreReceipt::query()->count());
        $this->deliveryRequest('reconcile', $this->pageRequest(true))->assertOk()
            ->assertJsonPath('operations.2.state', 'REJECTED')->assertJsonPath('operations.2.recovery_generation', '7');
    }

    public function test_store_receipt_closed_shapes_echoes_quantities_and_digest_mismatch_refuse_without_writes(): void
    {
        $this->seedDelivery();
        $receipt = $this->fixture('receipt.deliver-item.committed');
        foreach ([['binding_digest' => str_repeat('0', 64), 'status' => 409],
            ['binding_digest' => str_repeat('A', 64)], ['account_id' => (string) Str::uuid7()],
            ['entitlement_id' => (string) Str::uuid7()], ['product_id' => 'oteryn.wrong'], ['product_version' => '3'],
            ['delivered_quantity' => 9], ['delivered_quantity' => 0], ['result_code' => 'STORE_LINE_REVOKED'],
            ['outcome' => 'DEFERRED'], ['extra' => 1], ['recovery_generation' => '18446744073709551616'],
            ['decided_at' => '0'], ['issuer_authority' => 'OTHER']] as $bad) {
            $status = $bad['status'] ?? 400;
            unset($bad['status']);
            $this->deliveryRequest('receipt', array_replace($receipt, $bad))->assertStatus($status)->assertContent('');
        }
        $reordered = $receipt;
        unset($reordered['binding_digest']);
        $reordered['binding_digest'] = $receipt['binding_digest'];
        $this->deliveryRequest('receipt', $reordered)->assertStatus(400);
        $this->deliveryRequest('receipt', array_replace($receipt, ['operation_id' => (string) Str::uuid7()]))->assertNotFound();
        $this->deliveryRequest('receipt', array_replace($receipt, ['operation_kind' => 'CLAIM']))->assertStatus(400);
        self::assertSame(0, StoreReceipt::query()->count());
        self::assertSame(7, StoreOperation::query()->where('state', 'PENDING')->count());
        self::assertSame(3, StoreFulfilmentUnit::query()->where('state', 'DELIVERY_PENDING')->count());
    }

    public function test_store_all_fixture_receipts_dispatch_and_reclaim_requires_the_bound_claim_receipt(): void
    {
        $this->seedDelivery();
        // The RECLAIM receipt is not accepted until its bound CLAIM receipt exists.
        $reclaim = $this->fixture('receipt.reclaim.committed-shortfall');
        $this->deliveryRequest('receipt', $reclaim)->assertStatus(503)->assertContent('');
        foreach (['deliver-premium-time.committed', 'deliver-item.committed', 'claim.committed',
            'revoke-unclaimed.rejected-already-claimed', 'reclaim.committed-shortfall', 'flag.committed'] as $name) {
            $receipt = $this->fixture('receipt.'.$name);
            $raw = StoreWire::encode($receipt, 1024);
            $this->assertWireSchema($raw, 'receipt');
            $this->deliveryRequest('receipt', $raw)->assertOk();
            $this->deliveryRequest('receipt', $raw)->assertOk();
        }
        self::assertSame(6, StoreReceipt::query()->count());
        // Claims and reversals do not write delivery state in this lane.
        self::assertSame('DELIVERED', StoreFulfilmentUnit::query()->whereKey($reclaim['entitlement_id'])->value('state'));
    }

    public function test_store_wire_rule_fixtures_refuse_wrong_echo_quantity_and_reclaim_sum(): void
    {
        $this->seedDelivery();
        $this->deliveryRequest('receipt', $this->fixture('receipt.claim.committed'))->assertOk();
        $manifest = $this->fixture('manifest');
        self::assertIsArray($manifest['rule_cases']);
        foreach ($manifest['rule_cases'] as $case) {
            self::assertIsArray($case);
            if ($case['message'] === 'receipt') {
                self::assertIsArray($case['body']);
                /** @var array<string, mixed> $body */
                $body = $case['body'];
                self::assertIsInt($case['expected_status']);
                $this->deliveryRequest('receipt', $body)->assertStatus($case['expected_status'])->assertContent('');
            }
        }
        self::assertSame(1, StoreReceipt::query()->count());
    }

    public function test_store_receipt_dependency_failure_rolls_back_receipt_operation_and_unit_together(): void
    {
        $this->seedDelivery();
        $receipt = $this->fixture('receipt.deliver-item.committed');
        StoreFulfilmentUnit::updating(function (): never {
            throw new QueryException('pgsql', 'fixture failure', [], new PDOException('fixture failure'));
        });
        try {
            $response = $this->deliveryRequest('receipt', $receipt)->assertStatus(503)->assertContent('');
            self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        } finally {
            StoreFulfilmentUnit::flushEventListeners();
        }
        self::assertSame(0, StoreReceipt::query()->count());
        self::assertSame('PENDING', StoreOperation::query()->where('operation_id', $receipt['operation_id'])->value('state'));
        self::assertSame('DELIVERY_PENDING', StoreFulfilmentUnit::query()->whereKey($receipt['entitlement_id'])->value('state'));
        $this->deliveryRequest('receipt', $receipt)->assertOk();
    }

    public function test_store_wire_fixtures_match_closed_schemas_and_delivery_schema_failures_dispatch_as_400(): void
    {
        $manifest = $this->fixture('manifest');
        self::assertIsArray($manifest['valid']);
        foreach ($manifest['valid'] as $name => $metadata) {
            self::assertIsString($name);
            self::assertIsArray($metadata);
            self::assertIsString($metadata['schema']);
            self::assertIsInt($metadata['max_bytes']);
            $data = $this->fixture(substr($name, 0, -5));
            $raw = StoreWire::encode($data, $metadata['max_bytes']);
            $this->assertWireSchema($raw, substr($metadata['schema'], 0, -12));
            self::assertSame($metadata['canonical_bytes'], strlen($raw));
            self::assertSame($metadata['sha256'], hash('sha256', $raw));
        }
        self::assertIsArray($manifest['invalid_cases']);
        foreach ($manifest['invalid_cases'] as $case) {
            self::assertIsArray($case);
            $path = match ($case['schema'] ?? null) {
                'receipt.schema.json' => 'receipt', 'read-request.schema.json' => 'read',
                'pending-request.schema.json' => 'pending', 'reconcile-request.schema.json' => 'reconcile',
                default => null,
            };
            if ($path !== null && ($case['level'] ?? null) === 'schema') {
                self::assertIsArray($case['body']);
                /** @var array<string, mixed> $body */
                $body = $case['body'];
                $this->deliveryRequest($path, $body)->assertStatus(400)->assertContent('');
            }
        }
    }
}
