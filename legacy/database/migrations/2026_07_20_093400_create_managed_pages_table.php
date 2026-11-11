<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('managed_pages', function (Blueprint $table): void {
            $table->id();
            $table->string('slug', 160)->unique();
            $table->string('title', 200);
            $table->text('body');
            $table->timestamp('published_at')->nullable()->index();
            $table->timestamps();
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('managed_pages');
    }
};
