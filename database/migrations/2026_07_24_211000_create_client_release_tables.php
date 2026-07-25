<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('client_releases', function (Blueprint $table): void {
            $table->id();
            $table->string('version', 64);
            $table->string('channel', 16);
            $table->text('release_notes')->nullable();
            $table->timestamp('published_at')->nullable()->index();
            $table->boolean('is_current')->default(false);
            $table->timestamps();

            $table->unique(['channel', 'version']);
            $table->index(['channel', 'is_current', 'published_at'], 'client_releases_public_lookup');
        });

        Schema::create('client_release_artifacts', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('client_release_id')->constrained()->cascadeOnDelete();
            $table->string('platform', 24);
            $table->string('architecture', 24);
            $table->string('artifact_url', 2048);
            $table->string('filename', 255);
            $table->unsignedBigInteger('size_bytes');
            $table->char('sha256', 64);
            $table->boolean('is_enabled')->default(true);
            $table->timestamps();

            $table->unique(
                ['client_release_id', 'platform', 'architecture'],
                'client_release_artifact_variant_unique',
            );
            $table->index(['platform', 'is_enabled']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('client_release_artifacts');
        Schema::dropIfExists('client_releases');
    }
};
