<?php

namespace App\ProductsEntitlements\Store\Models;

use Illuminate\Database\Eloquent\Model;

/** @property string $canonical_bytes */
final class StorePriceListImport extends Model
{
    protected $table = 'store_price_list_imports';

    protected $primaryKey = 'catalog_revision';

    public $incrementing = false;

    protected $keyType = 'string';

    public $timestamps = false;

    protected $guarded = [];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'imported_at' => 'immutable_datetime',
        ];
    }
}
