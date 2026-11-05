<?php

namespace App\ProductsEntitlements\Store\Models;

use Illuminate\Database\Eloquent\Model;

/**
 * @property string $catalog_revision
 * @property string $offer_key
 * @property int $unit_price
 * @property int $max_quantity
 * @property bool $purchasable
 * @property string $canonical_bytes
 */
final class StoreOffer extends Model
{
    protected $table = 'store_offers';

    protected $primaryKey = 'id';

    public $incrementing = true;

    protected $keyType = 'int';

    public $timestamps = false;

    protected $guarded = [];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'unit_price' => 'integer',
            'max_quantity' => 'integer',
            'purchasable' => 'boolean',
        ];
    }
}
