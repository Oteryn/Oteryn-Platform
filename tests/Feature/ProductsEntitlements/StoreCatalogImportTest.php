<?php

namespace Tests\Feature\ProductsEntitlements;

use App\ProductsEntitlements\Store\ActiveStoreCatalog;
use App\ProductsEntitlements\Store\StoreCatalogImporter;
use App\ProductsEntitlements\Store\StorePriceList;
use Illuminate\Database\QueryException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use Tests\TestCase;

final class StoreCatalogImportTest extends TestCase
{
    use RefreshDatabase;

    private const FIXTURE = 'docs/contracts/fixtures/store-price-list-v1/price-list.valid.json';

    public function test_store_artisan_import_is_verbatim_idempotent_and_activation_is_explicit_exclusive(): void
    {
        self::assertSame('pgsql', DB::getDriverName(), 'Store qualification requires the PostgreSQL test wrapper.');
        $raw = $this->fixture();
        $revision = hash('sha256', $raw);
        self::assertSame(0, Artisan::call('store:catalog:import', ['file' => base_path(self::FIXTURE)]));
        self::assertSame($raw, DB::table('store_price_list_imports')->value('canonical_bytes'));
        self::assertSame($revision, DB::table('store_price_list_imports')->value('catalog_revision'));
        self::assertNull(DB::table('store_active_catalog')->value('catalog_revision'));
        $query = app(ActiveStoreCatalog::class);
        self::assertNull($query->offer($revision, 'test-supply'));
        self::assertSame(0, Artisan::call('store:catalog:import', ['file' => base_path(self::FIXTURE)]));
        self::assertSame(1, DB::table('store_price_list_imports')->count());
        self::assertSame(1, DB::table('store_offers')->count());
        self::assertSame(0, Artisan::call('store:catalog:activate', ['catalog_revision' => $revision]));
        $offer = $query->offer($revision, 'test-supply');
        self::assertNotNull($offer);
        self::assertSame(25, $offer->unit_price);
        self::assertSame('test-supply', $offer->offer_key);
        self::assertTrue($offer->purchasable);
        self::assertSame(10, $offer->max_quantity);
        self::assertStringContainsString('"unit_kind":"ITEM"', $offer->canonical_bytes);
        self::assertNull($query->offer($revision, 'missing'));
        $secondRaw = str_replace('"25"', '"30"', $raw);
        $second = app(StoreCatalogImporter::class)->import($secondRaw);
        self::assertNotSame($revision, $second);
        self::assertNotNull($query->offer($revision, 'test-supply'));
        self::assertSame(0, Artisan::call('store:catalog:activate', ['catalog_revision' => $second]));
        self::assertSame(1, DB::table('store_active_catalog')->count());
        self::assertSame($second, DB::table('store_active_catalog')->value('catalog_revision'));
        self::assertNull($query->offer($revision, 'test-supply'));
        self::assertSame(30, $query->offer($second, 'test-supply')?->unit_price);
        $activated = DB::table('store_active_catalog')->value('activated_at');
        self::assertSame(0, Artisan::call('store:catalog:activate', ['catalog_revision' => $second]));
        self::assertSame($activated, DB::table('store_active_catalog')->value('activated_at'));
        self::assertFalse(config('products-entitlements.store.purchase_enabled'));
        self::assertFalse(config('products-entitlements.store.delivery_enabled'));
    }

    public function test_store_invalid_files_and_unimported_activation_leave_no_state(): void
    {
        $files = glob(base_path('docs/contracts/fixtures/store-price-list-v1/price-list.invalid-*.json'));
        self::assertIsArray($files);
        self::assertCount(5, $files);
        foreach ($files as $path) {
            self::assertSame(1, Artisan::call('store:catalog:import', ['file' => $path]));
        }
        self::assertSame(1, Artisan::call('store:catalog:import', ['file' => base_path('missing-store-catalog.json')]));
        foreach (['bad', str_repeat('f', 64)] as $revision) {
            self::assertSame(1, Artisan::call('store:catalog:activate', ['catalog_revision' => $revision]));
        }
        self::assertSame(0, DB::table('store_price_list_imports')->count());
        self::assertSame(0, DB::table('store_offers')->count());
        self::assertNull(DB::table('store_active_catalog')->value('catalog_revision'));
    }

    public function test_store_noncanonical_duplicate_and_deep_json_inputs_are_rejected(): void
    {
        $raw = $this->fixture();
        foreach ([$raw."\n", ' '.$raw, "\xEF\xBB\xBF".$raw, '{"contract_version":1,"contract_version":1,'.substr($raw, 22),
            str_replace('test-supply', 'test\\u002dsupply', $raw),
            str_replace('"contract_version":1,"offers":', '"offers":', $raw),
            '{"offers":[],"contract_version":1}', str_repeat('[', 10).str_repeat(']', 10), '[]', '{}', '',
            str_repeat('a', StorePriceList::MAX_BYTES + 1)] as $invalid) {
            try {
                app(StoreCatalogImporter::class)->import($invalid);
                self::fail('Invalid Store price list was accepted.');
            } catch (InvalidArgumentException) {
                self::assertSame(0, DB::table('store_price_list_imports')->count());
            }
        }
    }

    public function test_store_supports_all_unit_kinds_and_transferable_price_provenance_without_wallet_mutation(): void
    {
        foreach (['ITEM', 'COSMETIC', 'PREMIUM_TIME'] as $kind) {
            $raw = str_replace(['"ITEM"', '"COINS"'], ['"'.$kind.'"', '"TRANSFERABLE_COINS"'], $this->fixture());
            app(StoreCatalogImporter::class)->import($raw);
        }
        self::assertSame(3, DB::table('store_offers')->where('currency', 'TRANSFERABLE_COINS')->count());
        self::assertSame(0, DB::table('wallet_accounts')->count());
        self::assertSame(0, DB::table('store_operations')->count());
    }

    public function test_store_catalog_constraints_reject_mutation_duplicate_keys_and_second_active_slot(): void
    {
        $revision = app(StoreCatalogImporter::class)->import($this->fixture());
        foreach ([fn () => DB::table('store_price_list_imports')->where('catalog_revision', $revision)->update(['canonical_bytes' => '{}']),
            fn () => DB::table('store_price_list_imports')->delete(),
            fn () => DB::table('store_offers')->update(['unit_price' => 1]),
            fn () => DB::table('store_offers')->delete(),
            fn () => DB::table('store_active_catalog')->insert(['id' => 2]),
            fn () => DB::table('store_active_catalog')->where('id', 1)->update(['catalog_revision' => str_repeat('f', 64), 'activated_at' => now()]),
            fn () => DB::table('store_price_list_imports')->insert(['catalog_revision' => str_repeat('f', 64), 'canonical_bytes' => '{}', 'imported_at' => now()])] as $mutation) {
            try {
                DB::transaction($mutation);
                self::fail('PostgreSQL accepted an invalid Store catalog mutation.');
            } catch (QueryException) {
                self::assertSame($this->fixture(), DB::table('store_price_list_imports')->value('canonical_bytes'));
            }
        }
    }

    private function fixture(): string
    {
        $raw = file_get_contents(base_path(self::FIXTURE));
        self::assertIsString($raw);

        return $raw;
    }
}
