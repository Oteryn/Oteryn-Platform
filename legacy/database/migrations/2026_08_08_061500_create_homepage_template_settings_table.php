<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('homepage_template_settings', function (Blueprint $table): void {
            $table->unsignedTinyInteger('id')->primary();
            $table->string('active_key', 64);
            $table->string('previous_key', 64)->nullable();
            $table->unsignedBigInteger('version')->default(0);
            $table->foreignId('updated_by_identity_id')->nullable()->constrained('identities')->nullOnDelete();
            $table->timestamps();
        });

        $now = now();

        DB::table('homepage_template_settings')->insert([
            'id' => 1,
            'active_key' => 'production',
            'previous_key' => null,
            'version' => 0,
            'updated_by_identity_id' => null,
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    public function down(): void
    {
        Schema::dropIfExists('homepage_template_settings');
    }
};
