<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('wiki_articles', function (Blueprint $table): void {
            $table->id();
            $table->string('content_type', 64);
            $table->string('status', 32)->default('draft')->index();
            $table->boolean('is_featured')->default(false);
            $table->unsignedInteger('sort_order')->default(0);
            $table->foreignId('author_identity_id')->nullable()->constrained('identities')->nullOnDelete();
            $table->foreignId('last_editor_identity_id')->nullable()->constrained('identities')->nullOnDelete();
            $table->foreignId('publisher_identity_id')->nullable()->constrained('identities')->nullOnDelete();
            $table->timestamp('published_at')->nullable()->index();
            $table->unsignedBigInteger('lock_version')->default(1);
            $table->timestamps();
        });

        Schema::create('wiki_article_translations', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('article_id')->constrained('wiki_articles')->cascadeOnDelete();
            $table->string('locale', 5);
            $table->string('title', 200);
            $table->string('slug', 160);
            $table->string('summary', 1000)->default('');
            $table->longText('source_markdown');
            $table->timestamps();
            $table->unique(['article_id', 'locale']);
            $table->unique(['locale', 'slug']);
        });

        Schema::create('wiki_categories', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('parent_id')->nullable()->constrained('wiki_categories')->restrictOnDelete();
            $table->string('key', 96)->unique();
            $table->unsignedInteger('sort_order')->default(0);
            $table->boolean('visible')->default(true);
            $table->unsignedBigInteger('lock_version')->default(1);
            $table->timestamps();
        });

        Schema::create('wiki_category_translations', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('category_id')->constrained('wiki_categories')->cascadeOnDelete();
            $table->string('locale', 5);
            $table->string('name', 200);
            $table->string('slug', 160);
            $table->text('description')->nullable();
            $table->timestamps();
            $table->unique(['category_id', 'locale']);
            $table->unique(['locale', 'slug']);
        });

        Schema::create('wiki_article_category', function (Blueprint $table): void {
            $table->foreignId('article_id')->constrained('wiki_articles')->cascadeOnDelete();
            $table->foreignId('category_id')->constrained('wiki_categories')->cascadeOnDelete();
            $table->unsignedInteger('sort_order')->default(0);
            $table->primary(['article_id', 'category_id']);
        });

        Schema::create('wiki_revisions', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('article_id')->constrained('wiki_articles')->cascadeOnDelete();
            $table->string('locale', 5);
            $table->unsignedInteger('revision_number');
            $table->unsignedBigInteger('article_version');
            $table->string('title', 200);
            $table->string('slug', 160);
            $table->string('summary', 1000)->default('');
            $table->longText('source_markdown');
            $table->foreignId('editor_identity_id')->nullable()->constrained('identities')->nullOnDelete();
            $table->string('change_note', 500)->nullable();
            $table->foreignId('source_revision_id')->nullable()->constrained('wiki_revisions')->restrictOnDelete();
            $table->timestamp('created_at');
            $table->unique(['article_id', 'locale', 'revision_number']);
            $table->index(['article_id', 'created_at']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('wiki_revisions');
        Schema::dropIfExists('wiki_article_category');
        Schema::dropIfExists('wiki_category_translations');
        Schema::dropIfExists('wiki_categories');
        Schema::dropIfExists('wiki_article_translations');
        Schema::dropIfExists('wiki_articles');
    }
};
