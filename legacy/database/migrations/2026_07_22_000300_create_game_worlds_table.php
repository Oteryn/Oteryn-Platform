<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('game_worlds', function (Blueprint $table): void {
            $table->id();
            $table->string('slug', 64)->unique();
            $table->string('name', 100);
            $table->string('region', 32);
            $table->string('status', 20)->default('unknown')->index();
            $table->boolean('login_enabled')->default(false)->index();
            $table->string('game_host', 255);
            $table->unsignedSmallInteger('game_port');
            $table->timestamps();
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('game_worlds');
    }
};
