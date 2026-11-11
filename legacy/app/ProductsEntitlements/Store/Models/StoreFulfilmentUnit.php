<?php

namespace App\ProductsEntitlements\Store\Models;

use Illuminate\Database\Eloquent\Model;

final class StoreFulfilmentUnit extends Model
{
    protected $table = 'store_fulfilment_units';

    protected $primaryKey = 'entitlement_id';

    public $incrementing = false;

    protected $keyType = 'string';

    public $timestamps = false;

    protected $guarded = [];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'product_version' => 'string',
            'grant_revision' => 'string',
            'quantity' => 'integer',
            'unit_index' => 'integer',
            'created_at' => 'immutable_datetime',
        ];
    }
}
