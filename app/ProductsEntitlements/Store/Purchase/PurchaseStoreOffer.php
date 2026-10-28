<?php

namespace App\ProductsEntitlements\Store\Purchase;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire;
use App\Identity\Models\Identity;
use App\ProductsEntitlements\Store\ActiveStoreCatalog;
use App\ProductsEntitlements\Store\StoreSettings;
use App\ProductsEntitlements\Store\StoreWire;
use App\Wallet\WalletMutator;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Log;
use Illuminate\Support\Str;
use RuntimeException;
use Symfony\Component\HttpFoundation\Response;

/**
 * @phpstan-import-type Binding from StorePurchaseInput
 * @phpstan-import-type Offer from \App\ProductsEntitlements\Store\StorePriceList
 */
final class PurchaseStoreOffer
{
    public function __construct(
        private readonly ActiveStoreCatalog $catalog,
        private readonly WalletMutator $wallets,
        private readonly StorePurchaseWriter $writer,
    ) {}

    /** @param Binding $input */
    public function execute(array $input, string $raw, StoreSettings $settings): Response
    {
        return DB::transaction(function () use ($input, $raw, $settings): Response {
            DB::statement("SET LOCAL lock_timeout = '3s'");
            DB::statement("SET LOCAL statement_timeout = '5s'");
            // Serialize even a first-seen id, without persisting a provisional answer.
            DB::select('SELECT pg_advisory_xact_lock(hashtextextended(?, 0))', ['store-purchase:'.$input['purchase_request_id']]);
            $stored = DB::table('store_purchase_requests')->where('purchase_request_id', $input['purchase_request_id'])->first();
            if ($stored !== null) {
                if ($stored->request_bytes !== $raw) {
                    Log::warning('products_entitlements.store_purchase_request_conflict');

                    return response('', 409);
                }

                if (! is_string($stored->answer_bytes)) {
                    throw new RuntimeException('Stored Store answer unavailable.');
                }

                return response($stored->answer_bytes, 200, ['Content-Type' => 'application/json']);
            }
            $identity = Identity::query()->where('account_id', $input['account_id'])->lockForUpdate()->first();
            if ($identity === null || $identity->disabled_at !== null || $identity->termination_requested_at !== null || $identity->terminated_at !== null
                || DB::table('store_fulfilment_units')->where('account_id', $input['account_id'])
                    ->whereIn('state', ['MANUAL_REVIEW', 'REVERSAL_PARTIAL'])->exists()) {
                return $this->answer($input, $raw, 'ACCOUNT_RESTRICTED');
            }
            // The shared pointer lock fences activation for the complete decision.
            $active = DB::table('store_active_catalog')->where('id', 1)->sharedLock()->first();
            if ($active === null || $active->catalog_revision === null) {
                throw new RuntimeException('Store catalog unavailable.');
            }
            if ($active->catalog_revision !== $input['catalog_revision']) {
                return $this->answer($input, $raw, 'STALE_QUOTE');
            }
            $offer = $this->catalog->offer($input['catalog_revision'], $input['offer_key']);
            if ($offer === null || ! $offer->purchasable) {
                return $this->answer($input, $raw, 'OFFER_UNAVAILABLE');
            }
            /** @var Offer $data Immutable imported schema-validated bytes. */
            $data = json_decode($offer->canonical_bytes, true, 8, JSON_THROW_ON_ERROR);
            if ($data['units'] === []) {
                return $this->answer($input, $raw, 'OFFER_UNAVAILABLE');
            }
            foreach ($data['units'] as $unit) {
                if (! in_array($unit['unit_kind'], $settings->enabledUnitKinds, true)) {
                    return $this->answer($input, $raw, 'PURCHASE_DISABLED');
                }
            }
            if ($data['currency'] !== $input['currency'] || $data['unit_price'] !== $input['unit_price']) {
                return $this->answer($input, $raw, 'STALE_QUOTE');
            }
            if ($input['currency'] !== 'COINS' || $input['quantity'] > $offer->max_quantity
                || $offer->unit_price > intdiv(PHP_INT_MAX, $input['quantity']) || count($data['units']) > 8) {
                return $this->answer($input, $raw, 'OFFER_UNAVAILABLE');
            }
            DB::select('SELECT pg_advisory_xact_lock(hashtextextended(?, 0))', ['store-pending-capacity']);
            if (count($data['units']) > $settings->maxPendingOperations || DB::table('store_operations')->where('operation_kind', 'DELIVER')->whereIn('state', ['PENDING', 'REOPENED'])
                ->offset(max(0, $settings->maxPendingOperations - count($data['units'])))->limit(1)->exists()) {
                throw new RuntimeException('Store pending capacity unavailable.');
            }
            $units = $data['units'];
            usort($units, static fn (array $left, array $right): int => strcmp($left['product_id'], $right['product_id'])
                ?: NativeAccountCharactersWire::compareUint64($left['product_version'], $right['product_version']));
            foreach ($units as &$unit) {
                if ($unit['quantity'] > intdiv(65535, $input['quantity'])) {
                    return $this->answer($input, $raw, 'OFFER_UNAVAILABLE');
                }
                $unit['quantity'] *= $input['quantity'];
            }
            unset($unit);
            $total = $offer->unit_price * $input['quantity'];
            // A refusal must not create a previously absent Wallet row.
            try {
                $wallet = DB::transaction(function () use ($identity, $input, $total) {
                    $wallet = $this->wallets->lock($identity->id);
                    if ($wallet->available_balance < $total) {
                        throw new StoreInsufficientBalance;
                    }
                    $this->wallets->applyLocked($wallet, 'store_purchase', -$total, 0, 'store-purchase:'.$input['purchase_request_id']);

                    return $wallet;
                });
            } catch (StoreInsufficientBalance) {
                return $this->answer($input, $raw, 'INSUFFICIENT_BALANCE');
            }
            $order = (string) Str::uuid7();
            $answerUnits = [];
            foreach ($units as $unit) {
                $answerUnits[] = ['unit_kind' => $unit['unit_kind'], 'entitlement_id' => (string) Str::uuid7(),
                    'delivery_operation_id' => (string) Str::uuid7()];
            }
            // Insert the final immutable answer before the order's immediate FK is checked.
            $response = $this->answer($input, $raw, 'ACCEPTED', ['order_id' => $order, 'currency' => 'COINS',
                'total_price' => (string) $total, 'available_after' => (string) $wallet->available_balance, 'units' => $answerUnits]);
            $this->writer->write($input, $units, $identity->id, $total, $order, $answerUnits);

            return $response;
        });
    }

    /** @param Binding $input
     * @param  array<string,mixed>  $extra
     */
    private function answer(array $input, string $raw, string $result, array $extra = []): Response
    {
        $bytes = StoreWire::encode(['contract_version' => 1, 'purchase_request_id' => $input['purchase_request_id'],
            'request_digest' => hash('sha256', $raw), 'result' => $result, ...$extra, 'decided_at' => (string) now()->timestamp], 2048);
        DB::table('store_purchase_requests')->insert([...array_diff_key($input, array_flip(['contract_version', 'operation', 'source_authority'])),
            'request_bytes' => $raw, 'request_digest' => hash('sha256', $raw), 'result' => $result,
            'answer_bytes' => $bytes, 'decided_at' => now()]);

        return response($bytes, 200, ['Content-Type' => 'application/json']);
    }
}
