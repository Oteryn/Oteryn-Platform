<?php

namespace App\ProductsEntitlements\Store\Purchase;

use App\ProductsEntitlements\Store\StoreWire;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;

/**
 * @phpstan-import-type Binding from StorePurchaseInput
 * @phpstan-import-type Unit from \App\ProductsEntitlements\Store\StorePriceList
 */
final class StorePurchaseWriter
{
    /**
     * @param  Binding  $input
     * @param  list<Unit>  $units
     * @param  list<array{unit_kind:string,entitlement_id:string,delivery_operation_id:string}>  $answerUnits
     */
    public function write(array $input, array $units, int $identityId, int $total, string $order, array $answerUnits): void
    {
        $line = (string) Str::uuid7();
        DB::table('store_orders')->insert(['order_id' => $order, 'purchase_request_id' => $input['purchase_request_id'],
            'identity_id' => $identityId, 'wallet_idempotency_key' => 'store-purchase:'.$input['purchase_request_id'],
            'currency' => 'COINS', 'total_price' => $total, 'created_at' => now()]);
        DB::table('store_order_lines')->insert(['line_id' => $line, 'order_id' => $order,
            'catalog_revision' => $input['catalog_revision'], 'offer_key' => $input['offer_key'],
            'quantity' => $input['quantity'], 'unit_price' => $input['unit_price']]);
        foreach ($units as $index => $unit) {
            $entitlement = $answerUnits[$index]['entitlement_id'];
            $operation = $answerUnits[$index]['delivery_operation_id'];
            DB::table('store_fulfilment_units')->insert(['entitlement_id' => $entitlement, 'line_id' => $line,
                'unit_index' => $index, 'account_id' => $input['account_id'], ...$unit,
                'state' => 'DELIVERY_PENDING', 'created_at' => now()]);
            // Allocate before encoding: immutable intent bytes include the actual outbox sequence.
            $sequence = DB::scalar("SELECT nextval(pg_get_serial_sequence('store_operations', 'source_revision'))");
            if (! is_int($sequence) && ! is_string($sequence)) {
                throw new \RuntimeException('Store outbox sequence unavailable.');
            }
            $intent = StoreWire::encode(['contract_version' => 1, 'operation_kind' => 'DELIVER',
                'issuer_authority' => 'OTERYN_PLATFORM_STORE_AUTHORITY', 'source_revision' => (string) $sequence,
                'operation_id' => $operation, 'order_id' => $order, 'entitlement_id' => $entitlement,
                'account_id' => $input['account_id'], 'product_id' => $unit['product_id'],
                'product_version' => $unit['product_version'], 'unit_kind' => $unit['unit_kind'],
                'catalog_revision' => $input['catalog_revision'], 'offer_key' => $input['offer_key'],
                'grant_key' => $unit['grant_key'], 'grant_revision' => $unit['grant_revision'], 'quantity' => $unit['quantity'],
                'issued_at_source' => (string) now()->timestamp, 'audience' => 'OTERYN_GAME_STORE'], 4096);
            DB::table('store_operations')->insert(['source_revision' => $sequence, 'operation_id' => $operation,
                'issuer_authority' => 'OTERYN_PLATFORM_STORE_AUTHORITY', 'operation_kind' => 'DELIVER',
                'entitlement_id' => $entitlement, 'intent_bytes' => $intent, 'binding_digest' => hash('sha256', $intent),
                'state' => 'PENDING', 'created_at' => now()]);
        }

    }
}
