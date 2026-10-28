<?php

namespace App\ProductsEntitlements\Store;

use App\ProductsEntitlements\Store\Models\StorePriceListImport;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use RuntimeException;

final class StoreCatalogActivator
{
    public function activate(string $revision): void
    {
        if (preg_match('/^[0-9a-f]{64}$/D', $revision) !== 1) {
            throw new InvalidArgumentException('Store catalog revision must be lowercase SHA-256 hex.');
        }
        DB::transaction(function () use ($revision): void {
            $active = DB::table('store_active_catalog')->where('id', 1)->lockForUpdate()->first();
            if ($active === null) {
                throw new RuntimeException('Store catalog activation lock is missing.');
            }
            if (! StorePriceListImport::query()->whereKey($revision)->exists()) {
                throw new InvalidArgumentException('Store catalog revision has not been imported.');
            }
            if ($active->catalog_revision !== $revision) {
                DB::table('store_active_catalog')->where('id', 1)->update([
                    'catalog_revision' => $revision, 'activated_at' => now(),
                ]);
            }
        });
    }
}
