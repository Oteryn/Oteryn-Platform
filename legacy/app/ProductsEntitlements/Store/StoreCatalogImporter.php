<?php

namespace App\ProductsEntitlements\Store;

use App\ProductsEntitlements\Store\Models\StorePriceListImport;
use Illuminate\Support\Facades\DB;
use RuntimeException;

final class StoreCatalogImporter
{
    public function __construct(private readonly StorePriceList $priceList) {}

    public function import(string $raw): string
    {
        $data = $this->priceList->validate($raw);
        $revision = hash('sha256', $raw);

        return DB::transaction(function () use ($data, $raw, $revision): string {
            $inserted = DB::table('store_price_list_imports')->insertOrIgnore([
                'catalog_revision' => $revision, 'canonical_bytes' => $raw, 'imported_at' => now(),
            ]);
            if ($inserted === 0) {
                $existing = StorePriceListImport::query()->find($revision);
                if (! $existing instanceof StorePriceListImport || ! hash_equals($existing->canonical_bytes, $raw)) {
                    throw new RuntimeException('Store catalog revision conflicts with stored bytes.');
                }

                return $revision;
            }
            foreach ($data['offers'] as $offer) {
                DB::table('store_offers')->insert([
                    'catalog_revision' => $revision, 'offer_key' => $offer['offer_key'], 'currency' => $offer['currency'],
                    'unit_price' => $offer['unit_price'], 'max_quantity' => $offer['max_quantity'],
                    'purchasable' => $offer['purchasable'], 'canonical_bytes' => StoreWire::encode($offer, StorePriceList::MAX_BYTES),
                ]);
            }

            return $revision;
        });
    }
}
