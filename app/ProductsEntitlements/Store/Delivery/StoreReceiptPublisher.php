<?php

namespace App\ProductsEntitlements\Store\Delivery;

use App\CharacterAuthority\Restore\GameRestoreReconciler;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire as Wire;
use App\ProductsEntitlements\Store\Models\StoreFulfilmentUnit;
use App\ProductsEntitlements\Store\Models\StoreOperation;
use App\ProductsEntitlements\Store\Models\StoreReceipt;
use App\ProductsEntitlements\Store\Models\StoreReconcileState;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Log;
use InvalidArgumentException;

/** @phpstan-import-type Receipt from StoreDeliveryRequest */
final class StoreReceiptPublisher
{
    public function __construct(private readonly StoreDeliveryRequest $requests) {}

    public function publish(string $raw): string
    {
        $receipt = $this->requests->receipt($raw);
        DB::transaction(function () use ($receipt, $raw): void {
            GameRestoreReconciler::lock(false);
            $operation = StoreOperation::query()->where('issuer_authority', $receipt['issuer_authority'])
                ->where('operation_id', $receipt['operation_id'])->where('operation_kind', $receipt['operation_kind'])
                ->lockForUpdate()->first();
            if ($operation === null) {
                $this->refuse($receipt, 404, 'store_receipt_without_intent');
            }
            $current = StoreReceipt::query()->where('issuer_authority', $receipt['issuer_authority'])
                ->where('operation_id', $receipt['operation_id'])->whereNull('superseded_by_restore_event_id')->first();
            if ($current !== null) {
                if ($current->receipt_bytes === $raw) {
                    return; // Exact republication writes nothing, including after a later restore.
                }
                $this->refuse($receipt, 409, 'store_receipt_conflict');
            }
            $generation = str_pad($receipt['recovery_generation'], 20, '0', STR_PAD_LEFT);
            if (DB::table(GameRestoreReconciler::NOTICES)->where('new_recovery_generation', '>', $generation)->exists()
                || StoreReconcileState::query()->where('operation_id', $receipt['operation_id'])
                    ->where('new_recovery_generation', '>', $receipt['recovery_generation'])->exists()) {
                $this->refuse($receipt, 409, 'store_receipt_lost_to_restore');
            }
            $bytes = $operation->intent_bytes;
            if (! hash_equals(hash('sha256', $bytes), $receipt['binding_digest'])) {
                $this->refuse($receipt, 409, 'store_receipt_conflict');
            }
            $intent = Wire::decodeExact($bytes, 4096);
            // Serializes different operation kinds of the same fulfilment unit as well.
            $unit = StoreFulfilmentUnit::query()->whereKey($operation->entitlement_id)->lockForUpdate()->first();
            if ($unit === null) {
                throw new StoreDeliveryRefused(503);
            }
            $this->assertBinding($receipt, $intent, $unit->quantity);
            $stored = StoreReceipt::query()->create([
                ...array_intersect_key($receipt, array_flip(['issuer_authority', 'operation_id', 'operation_kind',
                    'binding_digest', 'outcome', 'result_code', 'recovery_generation', 'decided_at'])),
                'receipt_bytes' => $raw, 'receipt_digest' => hash('sha256', $raw), 'received_at' => now(),
            ]);
            $operation->update(['state' => $receipt['outcome']]);
            // The foundation names APPLIED as DELIVERED and terminal rejection as MANUAL_REVIEW.
            // CLAIM/reversal business workflows remain outside this delivery lane.
            if ($receipt['operation_kind'] === 'DELIVER') {
                $unit->update(['state' => $receipt['outcome'] === 'COMMITTED' ? 'DELIVERED' : 'MANUAL_REVIEW']);
            }
            StoreReconcileState::query()->where('operation_id', $receipt['operation_id'])->whereNull('resolved_receipt_id')
                ->update(['resolved_receipt_id' => $stored->getKey()]);
        }, 3);

        return '{"contract_version":1,"result":"accepted"}';
    }

    /** @param Receipt $receipt
     * @param  array<string, mixed>  $intent
     */
    private function assertBinding(array $receipt, array $intent, int $unitQuantity): void
    {
        foreach (['account_id', 'entitlement_id', 'product_id', 'product_version'] as $field) {
            if (($intent[$field] ?? null) !== $receipt[$field]) {
                throw new InvalidArgumentException('Store receipt echo mismatch.');
            }
        }
        if ($receipt['outcome'] !== 'COMMITTED') {
            return;
        }
        $quantityField = match ($receipt['operation_kind']) {
            'DELIVER' => 'delivered_quantity', 'CLAIM' => 'minted_quantity', 'REVOKE_UNCLAIMED' => 'revoked_quantity',
            default => null,
        };
        if ($quantityField !== null) {
            $quantity = $receipt['operation_kind'] === 'REVOKE_UNCLAIMED' ? $unitQuantity : ($intent['quantity'] ?? null);
            if (($receipt[$quantityField] ?? null) !== $quantity) {
                throw new InvalidArgumentException('Store receipt quantity mismatch.');
            }
        }
        if ($receipt['operation_kind'] === 'RECLAIM') {
            $this->assertReclaim($receipt, $intent);
        }
    }

    /** @param Receipt $receipt
     * @param  array<string, mixed>  $intent
     */
    private function assertReclaim(array $receipt, array $intent): void
    {
        $claimId = Wire::uuid7($intent, 'claim_operation_id');
        $claimDigest = Wire::text($intent, 'claim_receipt_digest', '/^[0-9a-f]{64}$/D');
        $claimBytes = StoreReceipt::query()->where('operation_id', $claimId)->where('receipt_digest', $claimDigest)
            ->where('operation_kind', 'CLAIM')->where('outcome', 'COMMITTED')->value('receipt_bytes');
        if (! is_string($claimBytes)) {
            throw new StoreDeliveryRefused(503);
        }
        $claim = Wire::decodeExact($claimBytes, 1024);
        foreach (['account_id', 'entitlement_id', 'product_id', 'product_version'] as $field) {
            if (($claim[$field] ?? null) !== $receipt[$field]) {
                throw new InvalidArgumentException('Store reclaim claim binding mismatch.');
            }
        }
        $reclaimed = $receipt['reclaimed_quantity'] ?? 0;
        $shortfall = $receipt['shortfall_quantity'] ?? 0;
        if ($reclaimed + $shortfall !== ($claim['minted_quantity'] ?? null)
            || ($receipt['result_code'] === 'STORE_RECLAIMED') !== ($shortfall === 0)
            || ($receipt['character_id'] ?? null) !== ($claim['character_id'] ?? null)
            || ($receipt['world_id'] ?? null) !== ($claim['world_id'] ?? null)) {
            throw new InvalidArgumentException('Store reclaim receipt mismatch.');
        }
    }

    /** @param Receipt $receipt */
    private function refuse(array $receipt, int $status, string $event): never
    {
        Log::error('products_entitlements.'.$event, ['operation_id' => $receipt['operation_id'],
            'operation_kind' => $receipt['operation_kind'], 'result_code' => $receipt['result_code']]);
        throw new StoreDeliveryRefused($status);
    }
}
