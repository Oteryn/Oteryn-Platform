<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('product_catalog_products', function (Blueprint $table): void {
            $table->string('product_id', 96)->primary();
            $table->timestamp('created_at');
        });

        Schema::create('product_catalog_versions', function (Blueprint $table): void {
            $table->string('product_id', 96);
            $table->unsignedInteger('version');
            $table->char('delivery_profile', 1);
            $table->string('target_scope', 16);
            $table->char('currency', 3);
            $table->unsignedBigInteger('price_minor');
            $table->unsignedBigInteger('available_from')->nullable();
            $table->unsignedBigInteger('available_until')->nullable();
            $table->char('payload_sha256', 64);
            $table->timestamp('created_at');

            $table->primary(['product_id', 'version']);
            $table->foreign('product_id')
                ->references('product_id')
                ->on('product_catalog_products')
                ->restrictOnDelete();
        });

        Schema::create('product_catalog_presentations', function (Blueprint $table): void {
            $table->string('product_id', 96);
            $table->unsignedInteger('version');
            $table->char('locale', 2);
            $table->string('name', 120);
            $table->string('description', 1000);
            $table->timestamp('created_at');

            $table->primary(['product_id', 'version', 'locale']);
            $table->foreign(['product_id', 'version'])
                ->references(['product_id', 'version'])
                ->on('product_catalog_versions')
                ->restrictOnDelete();
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('product_catalog_presentations');
        Schema::dropIfExists('product_catalog_versions');
        Schema::dropIfExists('product_catalog_products');
    }
};
