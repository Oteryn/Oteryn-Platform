<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('store_price_list_imports', function (Blueprint $table): void {
            $table->char('catalog_revision', 64)->primary();
            $table->text('canonical_bytes');
            $table->timestampTz('imported_at');
        });
        Schema::create('store_offers', function (Blueprint $table): void {
            $table->id();
            $table->char('catalog_revision', 64);
            $table->string('offer_key', 64);
            $table->string('currency', 20);
            $table->bigInteger('unit_price');
            $table->integer('max_quantity');
            $table->boolean('purchasable');
            $table->text('canonical_bytes');
            $table->unique(['catalog_revision', 'offer_key']);
            $table->foreign('catalog_revision')->references('catalog_revision')->on('store_price_list_imports')->restrictOnDelete();
        });
        Schema::create('store_active_catalog', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->char('catalog_revision', 64)->nullable();
            $table->timestampTz('activated_at')->nullable();
            $table->foreign('catalog_revision')->references('catalog_revision')->on('store_price_list_imports')->restrictOnDelete();
        });
        DB::table('store_active_catalog')->insert(['id' => 1]);

        if (DB::getDriverName() === 'pgsql') {
            DB::statement("ALTER TABLE store_price_list_imports ADD CHECK (catalog_revision ~ '^[0-9a-f]{64}$' AND catalog_revision = encode(sha256(convert_to(canonical_bytes, 'UTF8')), 'hex') AND octet_length(canonical_bytes) BETWEEN 1 AND 16777216)");
            DB::statement("ALTER TABLE store_offers ADD CHECK (currency IN ('COINS', 'TRANSFERABLE_COINS') AND unit_price > 0 AND max_quantity BETWEEN 1 AND 100 AND offer_key ~ '^[a-z0-9][a-z0-9._-]{0,63}$')");
            DB::statement('ALTER TABLE store_active_catalog ADD CHECK (id = 1 AND ((catalog_revision IS NULL) = (activated_at IS NULL)))');
            DB::unprepared(<<<'SQL'
                CREATE OR REPLACE FUNCTION store_reject_catalog_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
                BEGIN RAISE EXCEPTION 'Imported Store catalog is immutable'; END;
                $$;
                CREATE TRIGGER store_import_immutable BEFORE UPDATE OR DELETE ON store_price_list_imports
                FOR EACH ROW EXECUTE FUNCTION store_reject_catalog_mutation();
                CREATE TRIGGER store_offer_immutable BEFORE UPDATE OR DELETE ON store_offers
                FOR EACH ROW EXECUTE FUNCTION store_reject_catalog_mutation();
                SQL);
        }
    }

    public function down(): void
    {
        Schema::dropIfExists('store_active_catalog');
        Schema::dropIfExists('store_offers');
        Schema::dropIfExists('store_price_list_imports');
        if (DB::getDriverName() === 'pgsql') {
            DB::statement('DROP FUNCTION IF EXISTS store_reject_catalog_mutation()');
        }
    }
};
