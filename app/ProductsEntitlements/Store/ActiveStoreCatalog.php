<?php

namespace App\ProductsEntitlements\Store;

use App\ProductsEntitlements\Store\Models\StoreOffer;
use Illuminate\Database\Query\Builder;

/** Purchase-lane query boundary: a stale revision never resolves to the new active offer. */
final class ActiveStoreCatalog
{
    /** @phpstan-impure Reads the operator-controlled active pointer. */
    public function offer(string $catalogRevision, string $offerKey): ?StoreOffer
    {
        return StoreOffer::query()
            ->where('catalog_revision', $catalogRevision)->where('offer_key', $offerKey)
            ->whereExists(fn (Builder $query) => $query->selectRaw('1')->from('store_active_catalog')
                ->where('id', 1)->whereColumn('store_active_catalog.catalog_revision', 'store_offers.catalog_revision'))
            ->first();
    }
}
