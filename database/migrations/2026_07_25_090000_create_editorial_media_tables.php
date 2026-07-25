<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('editorial_media', function (Blueprint $table): void {
            $table->id();
            $table->string('disk', 64);
            $table->string('storage_path', 255)->unique();
            $table->string('thumbnail_path', 255)->nullable()->unique();
            $table->string('original_name', 255)->nullable();
            $table->string('mime_type', 32);
            $table->string('extension', 8);
            $table->unsignedBigInteger('byte_size');
            $table->unsignedInteger('width');
            $table->unsignedInteger('height');
            $table->unsignedBigInteger('thumbnail_byte_size')->nullable();
            $table->unsignedInteger('thumbnail_width')->nullable();
            $table->unsignedInteger('thumbnail_height')->nullable();
            $table->char('sha256', 64)->index();
            $table->string('alt_text', 500);
            $table->foreignId('uploaded_by_identity_id')
                ->nullable()
                ->constrained('identities')
                ->nullOnDelete();
            $table->timestamps();
        });

        Schema::create('editorial_media_references', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('media_id')
                ->constrained('editorial_media')
                ->restrictOnDelete();
            $table->string('consumer', 32);
            $table->string('consumer_id', 64);
            $table->string('usage', 64);
            $table->timestamps();

            $table->unique(
                ['consumer', 'consumer_id', 'usage'],
                'editorial_media_reference_slot_unique',
            );
            $table->index('media_id');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('editorial_media_references');
        Schema::dropIfExists('editorial_media');
    }
};
