<?php

namespace Tests\Feature\Wiki;

use App\Wiki\Domain\WikiArticleStatus;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;

final class PublicWikiReadTest extends TestCase
{
    use RefreshDatabase;

    public function test_home_category_and_article_reads_are_published_visible_and_locale_isolated(): void
    {
        $category = $this->category('guides', true, 'Guides', 'guides', 'Poradniki', 'poradniki');
        $published = $this->article(
            WikiArticleStatus::PUBLISHED->value,
            now()->subMinute(),
            'Published guide',
            'published-guide',
            'Opublikowany poradnik',
            'opublikowany-poradnik',
        );
        $this->attach($published, $category);
        $this->article(WikiArticleStatus::DRAFT->value, null, 'Draft secret', 'draft-secret', 'Sekretny szkic', 'sekretny-szkic');
        $this->article(WikiArticleStatus::IN_REVIEW->value, null, 'Review secret', 'review-secret', 'Recenzja sekret', 'recenzja-sekret');
        $this->article(WikiArticleStatus::ARCHIVED->value, null, 'Archive secret', 'archive-secret', 'Archiwum sekret', 'archiwum-sekret');
        $this->article(WikiArticleStatus::PUBLISHED->value, now()->addHour(), 'Future secret', 'future-secret', 'Przyszły sekret', 'przyszly-sekret');

        $this->get('/en/wiki')
            ->assertOk()
            ->assertSeeText('Guides')
            ->assertSeeText('Published guide')
            ->assertDontSeeText('Draft secret')
            ->assertDontSeeText('Review secret')
            ->assertDontSeeText('Archive secret')
            ->assertDontSeeText('Future secret');

        $this->get('/pl/wiki/category/poradniki')
            ->assertOk()
            ->assertSeeText('Poradniki')
            ->assertSeeText('Opublikowany poradnik')
            ->assertDontSeeText('Published guide');

        $this->get('/en/wiki/published-guide')
            ->assertOk()
            ->assertSeeText('Published guide')
            ->assertSeeText('Safe article body');

        foreach (['draft-secret', 'review-secret', 'archive-secret', 'future-secret'] as $slug) {
            $this->get('/en/wiki/'.$slug)
                ->assertNotFound()
                ->assertDontSeeText('secret');
        }
    }

    public function test_missing_translation_and_hidden_category_never_fall_back_or_leak(): void
    {
        $hidden = $this->category('hidden', false, 'Hidden', 'hidden', 'Ukryte', 'ukryte');
        $article = $this->article(
            WikiArticleStatus::PUBLISHED->value,
            now()->subMinute(),
            'English only',
            'english-only',
            null,
            null,
        );
        $this->attach($article, $hidden);

        $this->get('/en/wiki/english-only')
            ->assertOk()
            ->assertSeeText('English only')
            ->assertDontSee('/pl/wiki/', false);

        $this->get('/pl/wiki/english-only')
            ->assertNotFound()
            ->assertDontSeeText('English only')
            ->assertDontSeeText('Safe article body');

        $this->get('/en/wiki/category/hidden')->assertNotFound();
        $this->get('/en/wiki')->assertDontSeeText('Hidden');
    }

    public function test_stale_polish_translations_are_excluded_without_english_fallback(): void
    {
        $category = $this->category('security', true, 'Security', 'security', 'Bezpieczeństwo', 'bezpieczenstwo');
        $article = $this->article(
            WikiArticleStatus::PUBLISHED->value,
            now()->subMinute(),
            'Account security',
            'account-security',
            'Bezpieczeństwo konta',
            'bezpieczenstwo-konta',
        );
        $this->attach($article, $category);

        DB::table('wiki_article_translations')
            ->where('article_id', $article)
            ->where('locale', 'en')
            ->update(['updated_at' => now()]);
        DB::table('wiki_article_translations')
            ->where('article_id', $article)
            ->where('locale', 'pl')
            ->update(['updated_at' => now()->subHour()]);
        DB::table('wiki_category_translations')
            ->where('category_id', $category)
            ->where('locale', 'en')
            ->update(['updated_at' => now()]);
        DB::table('wiki_category_translations')
            ->where('category_id', $category)
            ->where('locale', 'pl')
            ->update(['updated_at' => now()->subHour()]);

        $this->get('/en/wiki/account-security')
            ->assertOk()
            ->assertSeeText('Account security');

        $this->get('/pl/wiki')
            ->assertOk()
            ->assertDontSeeText('Bezpieczeństwo')
            ->assertDontSeeText('Account security');
        $this->get('/pl/wiki/bezpieczenstwo-konta')
            ->assertNotFound()
            ->assertDontSeeText('Account security');
        $this->get('/pl/wiki/category/bezpieczenstwo')->assertNotFound();
    }

    public function test_canonical_hreflang_breadcrumbs_toc_and_related_articles_are_deterministic(): void
    {
        $parent = $this->category('learn', true, 'Learn', 'learn', 'Wiedza', 'wiedza');
        $child = $this->category('start', true, 'Start', 'start', 'Początek', 'poczatek', $parent);
        $article = $this->article(
            WikiArticleStatus::PUBLISHED->value,
            now()->subMinutes(2),
            'First steps',
            'first-steps',
            'Pierwsze kroki',
            'pierwsze-kroki',
            "# Install\n\n## Install\n\nSafe article body",
        );
        $relatedA = $this->article(
            WikiArticleStatus::PUBLISHED->value,
            now()->subMinute(),
            'A related',
            'a-related',
            'A powiązany',
            'a-powiazany',
        );
        $relatedB = $this->article(
            WikiArticleStatus::PUBLISHED->value,
            now()->subMinute(),
            'B related',
            'b-related',
            'B powiązany',
            'b-powiazany',
        );
        $this->attach($article, $child);
        $this->attach($relatedB, $child);
        $this->attach($relatedA, $child);

        $this->get('/en/wiki/first-steps')
            ->assertOk()
            ->assertSee('<link rel="canonical" href="'.url('/en/wiki/first-steps').'">', false)
            ->assertSee('hreflang="pl" href="'.url('/pl/wiki/pierwsze-kroki').'"', false)
            ->assertSeeInOrder(['Learn', 'Start', 'First steps'])
            ->assertSee('href="#install"', false)
            ->assertSee('href="#install-2"', false)
            ->assertSeeInOrder(['A related', 'B related']);

        $this->get('/pl/wiki/pierwsze-kroki')
            ->assertOk()
            ->assertSeeText('Pierwsze kroki')
            ->assertDontSeeText('First steps')
            ->assertSee('hreflang="en" href="'.url('/en/wiki/first-steps').'"', false);

        $this->get('/wiki/first-steps')
            ->assertOk()
            ->assertHeader('Content-Language', 'en')
            ->assertSee('<link rel="canonical" href="'.url('/en/wiki/first-steps').'">', false);
    }

    public function test_empty_and_database_unavailable_states_are_truthful(): void
    {
        $this->get('/en/wiki')
            ->assertOk()
            ->assertSeeText('The Wiki has no published content yet.');

        Schema::drop('wiki_article_translations');

        $this->get('/en/wiki')
            ->assertStatus(503)
            ->assertSeeText('Wiki is temporarily unavailable.');
    }

    private function category(
        string $key,
        bool $visible,
        string $enName,
        string $enSlug,
        string $plName,
        string $plSlug,
        ?int $parentId = null,
    ): int {
        $id = DB::table('wiki_categories')->insertGetId([
            'parent_id' => $parentId,
            'key' => $key,
            'sort_order' => 0,
            'visible' => $visible,
            'lock_version' => 1,
            'created_at' => now(),
            'updated_at' => now(),
        ]);
        DB::table('wiki_category_translations')->insert([
            [
                'category_id' => $id,
                'locale' => 'en',
                'name' => $enName,
                'slug' => $enSlug,
                'description' => $enName.' description',
                'created_at' => now(),
                'updated_at' => now(),
            ],
            [
                'category_id' => $id,
                'locale' => 'pl',
                'name' => $plName,
                'slug' => $plSlug,
                'description' => $plName.' opis',
                'created_at' => now(),
                'updated_at' => now(),
            ],
        ]);

        return $id;
    }

    private function article(
        string $status,
        mixed $publishedAt,
        string $enTitle,
        string $enSlug,
        ?string $plTitle,
        ?string $plSlug,
        string $markdown = 'Safe article body',
    ): int {
        $id = DB::table('wiki_articles')->insertGetId([
            'content_type' => 'guide',
            'status' => $status,
            'is_featured' => true,
            'sort_order' => 0,
            'published_at' => $publishedAt,
            'lock_version' => 1,
            'created_at' => now(),
            'updated_at' => now(),
        ]);
        $translations = [[
            'article_id' => $id,
            'locale' => 'en',
            'title' => $enTitle,
            'slug' => $enSlug,
            'summary' => $enTitle.' summary',
            'source_markdown' => $markdown,
            'created_at' => now(),
            'updated_at' => now(),
        ]];
        if ($plTitle !== null && $plSlug !== null) {
            $translations[] = [
                'article_id' => $id,
                'locale' => 'pl',
                'title' => $plTitle,
                'slug' => $plSlug,
                'summary' => $plTitle.' podsumowanie',
                'source_markdown' => $markdown,
                'created_at' => now(),
                'updated_at' => now(),
            ];
        }
        DB::table('wiki_article_translations')->insert($translations);

        return $id;
    }

    private function attach(int $articleId, int $categoryId): void
    {
        DB::table('wiki_article_category')->insert([
            'article_id' => $articleId,
            'category_id' => $categoryId,
            'sort_order' => 0,
        ]);
    }
}
