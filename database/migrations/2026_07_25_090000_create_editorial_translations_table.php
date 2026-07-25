<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('editorial_translations', function (Blueprint $table): void {
            $table->id();
            $table->string('content_type', 40);
            $table->unsignedBigInteger('content_id');
            $table->string('locale', 8);
            $table->string('title', 200)->nullable();
            $table->text('body')->nullable();
            $table->string('action_label', 80)->nullable();
            $table->timestamp('source_updated_at');
            $table->timestamp('published_at')->nullable()->index();
            $table->timestamps();

            $table->unique(['content_type', 'content_id', 'locale'], 'editorial_translation_identity');
            $table->index(['content_type', 'locale', 'published_at'], 'editorial_translation_public_lookup');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('editorial_translations');
    }
};
