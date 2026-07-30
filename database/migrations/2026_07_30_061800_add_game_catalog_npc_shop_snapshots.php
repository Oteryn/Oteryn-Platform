<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('game_catalog_relation_snapshots', function (Blueprint $table): void {
            $table->string('availability', 32)->nullable()->after('completeness');
            $table->string('canonical_key', 320)->change();
        });

        Schema::create('game_catalog_npc_snapshots', function (Blueprint $table): void {
            $table->foreignId('entity_snapshot_id')->primary()->constrained('game_catalog_entity_snapshots')->cascadeOnDelete();
            $table->string('registry_key', 200)->index();
            $table->string('runtime_name', 200)->index();
            $table->string('display_name', 200)->nullable();
            $table->string('type_name', 200);
            $table->string('name_description', 500)->nullable();
            $table->json('aliases');
            $table->string('registration_status', 32)->index();
            $table->foreignId('currency_entity_id')->constrained('game_catalog_entities')->restrictOnDelete();
            $table->unsignedInteger('currency_server_id');
            $table->json('attributes');
        });

        Schema::create('game_catalog_shop_offer_snapshots', function (Blueprint $table): void {
            $table->foreignId('relation_snapshot_id')->primary()->constrained('game_catalog_relation_snapshots')->cascadeOnDelete();
            $table->string('direction', 8)->index();
            $table->foreignId('currency_entity_id')->constrained('game_catalog_entities')->restrictOnDelete();
            $table->unsignedInteger('currency_server_id');
            $table->json('runtime_path');
            $table->string('item_name', 200);
            $table->integer('item_subtype');
            $table->unsignedInteger('priced_item_count');
            $table->unsignedBigInteger('price_amount');
            $table->integer('storage_key')->nullable();
            $table->integer('storage_value')->nullable();
            $table->json('attributes');
            $table->index(['direction', 'currency_entity_id'], 'game_catalog_shop_offer_direction_currency_index');
        });
    }

    public function down(): void
    {
        if (Schema::hasTable('game_catalog_npc_snapshots') && DB::table('game_catalog_npc_snapshots')->exists()) {
            throw new \RuntimeException('Cannot roll back Game Catalog NPC/shop persistence while NPC snapshot rows exist.');
        }
        if (Schema::hasTable('game_catalog_shop_offer_snapshots') && DB::table('game_catalog_shop_offer_snapshots')->exists()) {
            throw new \RuntimeException('Cannot roll back Game Catalog NPC/shop persistence while shop-offer rows exist.');
        }

        Schema::dropIfExists('game_catalog_shop_offer_snapshots');
        Schema::dropIfExists('game_catalog_npc_snapshots');

        Schema::table('game_catalog_relation_snapshots', function (Blueprint $table): void {
            $table->dropColumn('availability');
            $table->string('canonical_key', 240)->change();
        });
    }
};
