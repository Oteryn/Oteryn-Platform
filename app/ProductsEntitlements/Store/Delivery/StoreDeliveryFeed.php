<?php

namespace App\ProductsEntitlements\Store\Delivery;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire as Wire;
use App\ProductsEntitlements\Store\Models\StoreOperation;
use App\ProductsEntitlements\Store\Models\StoreReconcileState;
use App\ProductsEntitlements\Store\StoreWire;
use Illuminate\Database\Query\Builder;
use Illuminate\Database\Query\JoinClause;

final class StoreDeliveryFeed
{
    public function __construct(private readonly StoreDeliveryRequest $requests) {}

    public function pending(string $raw): string
    {
        return $this->page($raw, false);
    }

    public function reconcile(string $raw): string
    {
        return $this->page($raw, true);
    }

    public function read(string $raw): string
    {
        $key = $this->requests->key($raw);
        $bytes = StoreOperation::query()->where('issuer_authority', $key['issuer_authority'])
            ->where('operation_kind', $key['operation_kind'])->where('operation_id', $key['operation_id'])
            ->value('intent_bytes');
        if (! is_string($bytes)) {
            throw new StoreDeliveryRefused(404);
        }

        return $bytes;
    }

    private function page(string $raw, bool $reconcile): string
    {
        $request = $this->requests->page($raw, $reconcile);
        $entries = [];
        // PostgreSQL bigint sequences cannot reach the upper half of a wire uint64 cursor.
        if (Wire::compareUint64($request['after_source_revision'], '9223372036854775807') <= 0) {
            $rows = StoreOperation::query()->toBase()
                ->leftJoin('store_receipts as receipt', function (JoinClause $join): void {
                    $join->on('receipt.operation_id', '=', 'store_operations.operation_id')
                        ->on('receipt.issuer_authority', '=', 'store_operations.issuer_authority')
                        ->whereNull('receipt.superseded_by_restore_event_id');
                })
                ->where('store_operations.issuer_authority', 'OTERYN_PLATFORM_STORE_AUTHORITY')
                ->when(! $reconcile, fn (Builder $query) => $query->whereNull('receipt.id'))
                ->where(function (Builder $query) use ($request): void {
                    $query->where('source_revision', '>', $request['after_source_revision'])
                        ->orWhere(fn (Builder $same) => $same->where('source_revision', $request['after_source_revision'])
                            ->where('store_operations.operation_id', '>', $request['after_operation_id']));
                })
                ->select(['store_operations.operation_kind', 'store_operations.operation_id', 'source_revision',
                    'receipt.outcome', 'receipt.receipt_digest', 'receipt.recovery_generation'])
                ->selectSub(StoreReconcileState::query()->selectRaw('1')
                    ->whereColumn('store_reconcile_states.operation_id', 'store_operations.operation_id')
                    ->whereNull('resolved_receipt_id')->limit(1), 'reopened')
                ->orderBy('source_revision')->orderBy('store_operations.operation_id')
                ->limit($request['max_entries'])->get();
            foreach ($rows as $row) {
                /** @var object{operation_kind:string,operation_id:string,source_revision:int,outcome:?string,receipt_digest:?string,recovery_generation:?string,reopened:?int} $row */
                $entry = ['operation_kind' => $row->operation_kind, 'operation_id' => $row->operation_id,
                    'source_revision' => (string) $row->source_revision];
                if ($reconcile) {
                    $entry += ['state' => $row->outcome ?? ($row->reopened !== null ? 'REOPENED' : 'PENDING'),
                        'receipt_digest' => $row->receipt_digest, 'recovery_generation' => $row->recovery_generation];
                }
                $entries[] = $entry;
            }
        }

        return StoreWire::encode(['contract_version' => 1, $reconcile ? 'operations' : 'pending' => $entries], $reconcile ? 12288 : 8192);
    }
}
