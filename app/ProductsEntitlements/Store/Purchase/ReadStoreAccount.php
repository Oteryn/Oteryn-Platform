<?php

namespace App\ProductsEntitlements\Store\Purchase;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire;
use App\Identity\Models\Identity;
use App\ProductsEntitlements\Store\StoreWire;
use App\Wallet\Models\WalletAccount;
use App\Wallet\Models\WalletLedgerEntry;
use Illuminate\Support\Facades\DB;
use Symfony\Component\HttpFoundation\Response;

final class ReadStoreAccount
{
    /** @param array{account_id:string,before_entry_id?:string,max_entries?:int} $input */
    public function read(array $input, bool $history): Response
    {
        $identity = Identity::query()->where('account_id', $input['account_id'])->first();
        if ($identity === null) {
            return response('', 404);
        }
        $body = ['contract_version' => 1, 'result' => 'OK', 'account_id' => $input['account_id']];
        if (! $history) {
            $wallet = WalletAccount::query()->find($identity->id);
            $body['balances'] = [['currency' => 'COINS', 'available' => (string) ($wallet->available_balance ?? 0),
                'reserved' => (string) ($wallet->reserved_balance ?? 0)]];
            $body['read_at'] = (string) now()->timestamp;
        } else {
            $query = WalletLedgerEntry::query()->where('identity_id', $identity->id);
            $before = $input['before_entry_id'] ?? '0';
            // Ledger ids are signed PostgreSQL bigint. Larger valid wire cursors mean all entries.
            if ($before !== '0' && NativeAccountCharactersWire::compareUint64($before, (string) PHP_INT_MAX) <= 0) {
                $query->where('id', '<', $before);
            }
            $max = $input['max_entries'] ?? 32;
            $rows = $query->orderByDesc('id')->limit($max + 1)->get();
            $body['entries'] = [];
            foreach ($rows->take($max) as $row) {
                $order = DB::table('store_orders')->join('store_order_lines', 'store_order_lines.order_id', '=', 'store_orders.order_id')
                    ->where('store_orders.identity_id', $identity->id)->where('wallet_idempotency_key', $row->idempotency_key)
                    ->select('store_orders.order_id', 'offer_key', 'quantity')->first();
                $kind = $order !== null && $row->operation_type === 'store_purchase' ? 'STORE_PURCHASE' : match ($row->operation_type) {
                    'administrator_adjustment' => 'OPERATOR_ADJUSTMENT',
                    'auction_bid_reserved' => 'AUCTION_RESERVE',
                    'auction_bid_released' => 'AUCTION_RELEASE',
                    'auction_purchase_settled', 'auction_sale_proceeds' => 'AUCTION_SETTLE',
                    'coin_package_credit' => 'COIN_PACKAGE_CREDIT',
                    'coin_package_reversal' => 'COIN_PACKAGE_REVERSAL',
                    default => 'OTHER',
                };
                $store = $kind === 'STORE_PURCHASE';
                $body['entries'][] = ['entry_id' => (string) $row->id, 'kind' => $kind, 'currency' => 'COINS',
                    'available_delta' => (string) $row->available_delta, 'reserved_delta' => (string) $row->reserved_delta,
                    'order_id' => $store ? $order->order_id : null, 'offer_key' => $store ? $order->offer_key : null,
                    'quantity' => $store ? $order->quantity : null, 'created_at' => (string) $row->created_at->timestamp];
            }
            $body['next_before_entry_id'] = $rows->count() > $max ? (string) $rows->take($max)->last()?->id : '0';
        }

        return response(StoreWire::encode($body, $history ? 12288 : 512), 200, ['Content-Type' => 'application/json']);
    }
}
