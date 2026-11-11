<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('events', function (Blueprint $table): void {
            $table->id();
            $table->string('status', 24)->index();
            $table->timestamp('starts_at')->index();
            $table->timestamp('ends_at')->index();
            $table->boolean('featured')->default(false)->index();
            $table->foreignId('news_post_id')->nullable()->constrained('news_posts')->nullOnDelete();
            $table->foreignId('created_by')->nullable()->constrained('identities')->nullOnDelete();
            $table->foreignId('updated_by')->nullable()->constrained('identities')->nullOnDelete();
            $table->foreignId('published_by')->nullable()->constrained('identities')->nullOnDelete();
            $table->unsignedInteger('lock_version')->default(1);
            $table->timestamps();

            $table->index(['status', 'starts_at', 'ends_at'], 'events_public_calendar_lookup');
        });

        Schema::create('event_translations', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('event_id')->constrained('events')->cascadeOnDelete();
            $table->string('locale', 8);
            $table->string('title', 200);
            $table->string('slug', 160);
            $table->string('summary', 500);
            $table->text('body');
            $table->timestamps();

            $table->unique(['event_id', 'locale']);
            $table->unique(['locale', 'slug']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('event_translations');
        Schema::dropIfExists('events');
    }
};
