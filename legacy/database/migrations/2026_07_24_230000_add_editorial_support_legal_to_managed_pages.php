<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('managed_pages', function (Blueprint $table): void {
            $table->string('legal_version', 40)->nullable()->after('body');
            $table->date('legal_effective_date')->nullable()->after('legal_version');
        });

        Schema::create('managed_page_legal_versions', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('managed_page_id')->constrained('managed_pages')->cascadeOnDelete();
            $table->string('version', 40);
            $table->date('effective_date');
            $table->string('title', 200);
            $table->text('body');
            $table->timestamp('published_at');
            $table->timestamps();

            $table->unique(['managed_page_id', 'version']);
            $table->index(['managed_page_id', 'effective_date']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('managed_page_legal_versions');

        Schema::table('managed_pages', function (Blueprint $table): void {
            $table->dropColumn(['legal_version', 'legal_effective_date']);
        });
    }
};
