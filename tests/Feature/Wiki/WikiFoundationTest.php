<?php

namespace Tests\Feature\Wiki;

use App\Admin\AdminPermission;
use App\Identity\Models\Identity;
use App\Wiki\Application\WikiArticleService;
use App\Wiki\Application\WikiCategoryService;
use App\Wiki\Domain\Exceptions\StaleWikiEdit;
use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Domain\WikiCategoryTranslationInput;
use App\Wiki\Domain\WikiTranslationInput;
use App\Wiki\Infrastructure\Audit\WikiAuditAction;
use App\Wiki\Infrastructure\Factories\WikiArticleFactory;
use App\Wiki\Infrastructure\Factories\WikiArticleTranslationFactory;
use App\Wiki\Infrastructure\Factories\WikiCategoryFactory;
use App\Wiki\Infrastructure\Factories\WikiCategoryTranslationFactory;
use App\Wiki\Infrastructure\Factories\WikiRevisionFactory;
use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiArticleTranslation;
use App\Wiki\Infrastructure\Models\WikiRevision;
use DomainException;
use Illuminate\Database\QueryException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Route;
use Illuminate\Support\Facades\Schema;
use InvalidArgumentException;
use LogicException;
use Tests\TestCase;

final class WikiFoundationTest extends TestCase
{
    use RefreshDatabase;

    public function test_migration_is_reversible_and_factories_create_valid_records(): void
    {
        $this->assertWikiTablesExist();

        $article = WikiArticleFactory::new()->create();
        $translation = WikiArticleTranslationFactory::new()->create(['article_id' => $article->id]);
        $category = WikiCategoryFactory::new()->create();
        $categoryTranslation = WikiCategoryTranslationFactory::new()->create(['category_id' => $category->id]);
        $revision = WikiRevisionFactory::new()->create(['article_id' => $article->id]);

        self::assertSame($article->id, $translation->article_id);
        self::assertSame($category->id, $categoryTranslation->category_id);
        self::assertSame($article->id, $revision->article_id);

        $migration = require database_path('migrations/2026_07_24_231000_create_wiki_foundation_tables.php');
        $down = [$migration, 'down'];
        self::assertIsCallable($down);
        $down();

        self::assertFalse(Schema::hasTable('wiki_articles'));
        self::assertFalse(Schema::hasTable('wiki_revisions'));

        $up = [$migration, 'up'];
        self::assertIsCallable($up);
        $up();
        $this->assertWikiTablesExist();
    }

    public function test_supported_locale_and_localized_slug_constraints_are_enforced(): void
    {
        $first = WikiArticleFactory::new()->create();
        $second = WikiArticleFactory::new()->create();
        $third = WikiArticleFactory::new()->create();

        WikiArticleTranslationFactory::new()->create([
            'article_id' => $first->id,
            'locale' => 'en',
            'slug' => 'shared-slug',
        ]);
        WikiArticleTranslationFactory::new()->create([
            'article_id' => $second->id,
            'locale' => 'pl',
            'slug' => 'shared-slug',
        ]);

        try {
            WikiArticleTranslationFactory::new()->create([
                'article_id' => $third->id,
                'locale' => 'en',
                'slug' => 'shared-slug',
            ]);
            self::fail('Expected localized Wiki article slug uniqueness to be database-enforced.');
        } catch (QueryException) {
            self::assertDatabaseCount('wiki_article_translations', 2);
        }

        $this->expectException(InvalidArgumentException::class);
        WikiArticleTranslation::query()->create([
            'article_id' => $third->id,
            'locale' => 'de',
            'title' => 'Unsupported',
            'slug' => 'unsupported',
            'summary' => 'Unsupported',
            'source_markdown' => '# Unsupported',
        ]);
    }

    public function test_article_writes_append_revisions_restore_as_new_and_reject_stale_edits(): void
    {
        $actor = $this->authorizedIdentity('wiki-editor@example.com', [
            AdminPermission::MANAGE_WIKI_ARTICLES,
            AdminPermission::PUBLISH_WIKI,
        ]);
        $service = $this->app->make(WikiArticleService::class);

        $article = $service->create($actor, 'guide', [
            $this->articleTranslation('en', 'Original English', 'restore-en', 'Original source'),
        ]);
        $source = WikiRevision::query()->where('article_id', $article->id)->firstOrFail();

        $article = $service->update($actor, $article, 1, 'guide', [
            $this->articleTranslation('en', 'Changed English', 'restore-en', 'Changed source'),
        ]);

        try {
            $service->update($actor, $article, 1, 'guide', [
                $this->articleTranslation('en', 'Stale overwrite', 'restore-en', 'Stale source'),
            ]);
            self::fail('Expected stale Wiki edit to fail.');
        } catch (StaleWikiEdit) {
            self::assertSame('# Changed source', WikiArticleTranslation::query()
                ->where('article_id', $article->id)
                ->where('locale', 'en')
                ->value('source_markdown'));
            self::assertDatabaseCount('wiki_revisions', 2);
        }

        $article = $service->restoreRevision($actor, $article, 2, $source, 'Restore original');
        $restored = WikiRevision::query()
            ->where('article_id', $article->id)
            ->orderByDesc('revision_number')
            ->firstOrFail();

        self::assertSame(3, $article->lock_version);
        self::assertSame(3, $restored->revision_number);
        self::assertSame($source->id, $restored->source_revision_id);
        self::assertSame('# Original source', $restored->source_markdown);
        self::assertSame('# Original source', WikiRevision::query()->findOrFail($source->id)->source_markdown);
    }

    public function test_revisions_are_append_only_through_the_supported_model(): void
    {
        $revision = WikiRevisionFactory::new()->create();

        try {
            $revision->forceFill(['title' => 'Mutated'])->save();
            self::fail('Expected Wiki revision update to fail.');
        } catch (LogicException) {
            self::assertSame('Factory revision', WikiRevision::query()->findOrFail($revision->id)->title);
        }

        $this->expectException(LogicException::class);
        WikiRevision::query()->findOrFail($revision->id)->delete();
    }

    public function test_publication_requires_complete_english_and_polish_content(): void
    {
        $actor = $this->authorizedIdentity('wiki-publisher@example.com', [
            AdminPermission::MANAGE_WIKI_ARTICLES,
            AdminPermission::PUBLISH_WIKI,
        ]);
        $service = $this->app->make(WikiArticleService::class);

        $article = $service->create($actor, 'guide', [
            $this->articleTranslation('en', 'English only', 'english-only', 'English source'),
        ]);
        $article = $service->submitForReview($actor, $article, 1);

        try {
            $service->publish($actor, $article, 2);
            self::fail('Expected publication without Polish content to fail.');
        } catch (DomainException) {
            self::assertSame(WikiArticleStatus::IN_REVIEW, WikiArticle::query()->findOrFail($article->id)->status);
        }

        $article = $service->update($actor, $article, 2, 'guide', [
            $this->articleTranslation('pl', 'Polska wersja', 'polska-wersja', 'Polska treść'),
        ]);
        $article = $service->publish($actor, $article, 3);

        self::assertSame(WikiArticleStatus::PUBLISHED, $article->status);
        self::assertNotNull($article->published_at);

        $this->expectException(DomainException::class);
        $service->update($actor, $article, 4, 'guide', [
            $this->articleTranslation('en', 'Direct published edit', 'english-only', 'Not allowed'),
        ]);
    }

    public function test_category_localized_slugs_and_stale_edits_are_enforced(): void
    {
        $actor = $this->authorizedIdentity('wiki-category-editor@example.com', [
            AdminPermission::MANAGE_WIKI_CATEGORIES,
        ]);
        $service = $this->app->make(WikiCategoryService::class);

        $category = $service->create($actor, 'getting-started', [
            new WikiCategoryTranslationInput('en', 'Getting Started', 'getting-started', 'Start here.'),
            new WikiCategoryTranslationInput('pl', 'Pierwsze kroki', 'pierwsze-kroki', 'Zacznij tutaj.'),
        ]);
        $category = $service->update($actor, $category, 1, 'getting-started', [
            new WikiCategoryTranslationInput('en', 'Start Here', 'getting-started', 'Updated.'),
        ]);

        try {
            $service->update($actor, $category, 1, 'getting-started', [
                new WikiCategoryTranslationInput('en', 'Stale', 'getting-started', 'Stale.'),
            ]);
            self::fail('Expected stale Wiki category edit to fail.');
        } catch (StaleWikiEdit) {
            self::assertSame('Start Here', DB::table('wiki_category_translations')
                ->where('category_id', $category->id)
                ->where('locale', 'en')
                ->value('name'));
        }

        $other = WikiCategoryFactory::new()->create();
        $this->expectException(QueryException::class);
        WikiCategoryTranslationFactory::new()->create([
            'category_id' => $other->id,
            'locale' => 'en',
            'slug' => 'getting-started',
        ]);
    }

    public function test_audit_metadata_is_bounded_and_public_routes_are_not_activated(): void
    {
        $actor = $this->authorizedIdentity('wiki-audit@example.com', [
            AdminPermission::MANAGE_WIKI_ARTICLES,
        ]);
        $secretMarker = 'FULL-ARTICLE-BODY-MUST-NOT-BE-AUDITED';
        $article = $this->app->make(WikiArticleService::class)->create($actor, 'guide', [
            $this->articleTranslation('en', 'Audited', 'audited', $secretMarker),
        ]);

        $event = DB::table('admin_audit_events')
            ->where('action', WikiAuditAction::ARTICLE_CREATED)
            ->where('target_id', (string) $article->id)
            ->first();

        self::assertNotNull($event);
        self::assertIsString($event->metadata);
        self::assertLessThanOrEqual(256, strlen($event->metadata));
        self::assertStringNotContainsString($secretMarker, $event->metadata);
        self::assertSame([
            'status' => 'draft',
            'version' => 1,
            'locales' => 'en',
        ], json_decode($event->metadata, true, flags: JSON_THROW_ON_ERROR));

        self::assertFalse(Route::has('wiki.index'));
        self::assertFalse(Route::has('wiki.article.show'));
        $this->get('/wiki')->assertNotFound();
    }

    private function assertWikiTablesExist(): void
    {
        foreach ([
            'wiki_articles',
            'wiki_article_translations',
            'wiki_categories',
            'wiki_category_translations',
            'wiki_article_category',
            'wiki_revisions',
        ] as $table) {
            self::assertTrue(Schema::hasTable($table));
        }
    }

    /**
     * @param  list<string>  $permissions
     */
    private function authorizedIdentity(string $email, array $permissions): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => 'wiki-test-role-'.str_replace(['@', '.'], '-', $email),
            'name' => 'Wiki test role',
            'created_at' => now(),
            'updated_at' => now(),
        ]);

        foreach ($permissions as $permission) {
            $permissionId = DB::table('admin_permissions')->where('key', $permission)->value('id');
            self::assertIsInt($permissionId);

            DB::table('admin_role_permissions')->insert([
                'role_id' => $roleId,
                'permission_id' => $permissionId,
            ]);
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);

        return $identity;
    }

    private function articleTranslation(
        string $locale,
        string $title,
        string $slug,
        string $source,
    ): WikiTranslationInput {
        return new WikiTranslationInput(
            $locale,
            $title,
            $slug,
            $title.' summary',
            '# '.$source,
        );
    }
}
