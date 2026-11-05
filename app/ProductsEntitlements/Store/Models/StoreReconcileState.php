<?php

namespace App\ProductsEntitlements\Store\Models;

use Illuminate\Database\Eloquent\Model;

final class StoreReconcileState extends Model
{
    protected $table = 'store_reconcile_states';

    protected $primaryKey = 'id';

    public $incrementing = true;

    protected $keyType = 'int';

    public $timestamps = false;

    protected $guarded = [];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'restored_generation' => 'string',
            'new_recovery_generation' => 'string',
            'superseded_receipt_id' => 'integer',
            'resolved_receipt_id' => 'integer',
        ];
    }
}
