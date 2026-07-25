<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('site_announcements', function (Blueprint $table): void {
            $table->id();
            $table->string('title', 200);
            $table->text('body');
            $table->string('severity', 24);
            $table->timestamp('starts_at')->index();
            $table->timestamp('ends_at')->nullable()->index();
            $table->string('publication_state', 24)->index();
            $table->string('action_label', 80)->nullable();
            $table->string('action_url', 2048)->nullable();
            $table->foreignId('created_by')->nullable()->constrained('identities')->nullOnDelete();
            $table->foreignId('updated_by')->nullable()->constrained('identities')->nullOnDelete();
            $table->foreignId('published_by')->nullable()->constrained('identities')->nullOnDelete();
            $table->unsignedInteger('lock_version')->default(1);
            $table->timestamps();

            $table->index(['publication_state', 'starts_at', 'ends_at'], 'site_announcements_active_lookup');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('site_announcements');
    }
};
