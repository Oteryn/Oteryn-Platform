<?php

namespace Tests\Feature\Wiki;

use App\Wiki\Domain\WikiArticleStatus;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Tests\TestCase;

final class PublicWikiSearchTest extends TestCase
{
    use RefreshDatabase;

    public function test_search_is_published_only_locale_isolated_and_deterministically_ranked(): void
    {
        $this->article('Dragon', 'dragon', 'Smok', 'smok', 'Exact dragon body');
        $this->article('Dragon lore', 'dragon-lore', 'Wiedza o smoku', 'wiedza-o-smoku', 'Dragon reference');
        $this->article('Other', 'other', 'Inny', 'inny', 'Contains dragon in the body');
        $this->article('Draft dragon', 'draft-dragon', 'Szkic smoka', 'szkic-smoka', 'Dragon secret', WikiArticleStatus::DRAFT->value);
        $this->article('English phoenix', 'english-phoenix', null, null, 'Phoenix only');

        $this->get('/en/wiki/search?q=Dragon')
            ->assertOk()
            ->assertSeeInOrder(['Dragon</a>', 'Dragon lore</a>', 'Other</a>'], false)
            ->assertDontSeeText('Draft dragon')
            ->assertDontSeeText('Smok');

        $this->get('/pl/wiki/search?q=smok')
            ->assertOk()
            ->assertSeeText('Smok')
            ->assertSeeText('Wiedza o smoku')
            ->assertDontSeeText('Dragon lore')
            ->assertDontSeeText('English phoenix');
    }

    public function test_search_is_bounded_paginated_and_preserves_literal_wildcards(): void
    {
        foreach (range(1, 13) as $index) {
            $this->article(
                sprintf('Guide %02d', $index),
                sprintf('guide-%02d', $index),
                sprintf('Poradnik %02d', $index),
                sprintf('poradnik-%02d', $index),
                'Shared bounded term',
            );
        }
        $this->article('Percent title', 'percent-title', 'Procent', 'procent', 'Literal 100% marker');

        $this->get('/en/wiki/search?q=bounded')
            ->assertOk()
            ->assertSeeText('13 results')
            ->assertSeeText('Page 1 of 2')
            ->assertSeeText('Guide 01')
            ->assertDontSeeText('Guide 13');

        $this->get('/en/wiki/search?q=bounded&page=2')
            ->assertOk()
            ->assertSeeText('Guide 13')
            ->assertSeeText('Page 2 of 2');

        $this->get('/en/wiki/search?q=100%')
            ->assertOk()
            ->assertSeeText('Percent title')
            ->assertSeeText('1 result');

        $this->get('/en/wiki/search?q=x')
            ->assertStatus(422)
            ->assertSeeText('The Wiki search query must contain at least two characters.');

        $this->get('/en/wiki/search?q='.str_repeat('x', 81))
            ->assertStatus(422)
            ->assertSeeText('The Wiki search query is too long.');

        $this->get('/pl/wiki/search?q=x')
            ->assertStatus(422)
            ->assertSeeText('Zapytanie wyszukiwania Wiki musi zawierać co najmniej dwa znaki.')
            ->assertDontSeeText('The Wiki search query must contain at least two characters.');
    }

    public function test_search_is_rate_limited_by_locale_and_source(): void
    {
        foreach (range(1, 30) as $attempt) {
            $this->get('/en/wiki/search?q=guide')->assertOk();
        }

        $this->get('/en/wiki/search?q=guide')->assertStatus(429);
        $this->get('/pl/wiki/search?q=guide')->assertOk();
    }

    public function test_search_excludes_stale_polish_translations(): void
    {
        $article = $this->article(
            'Account security',
            'account-security',
            'Bezpieczeństwo konta',
            'bezpieczenstwo-konta',
            'Security guidance',
        );
        DB::table('wiki_article_translations')
            ->where('article_id', $article)
            ->where('locale', 'en')
            ->update(['updated_at' => now()]);
        DB::table('wiki_article_translations')
            ->where('article_id', $article)
            ->where('locale', 'pl')
            ->update(['updated_at' => now()->subHour()]);

        $this->get('/en/wiki/search?q=security')
            ->assertOk()
            ->assertSeeText('Account security');
        $this->get('/pl/wiki/search?q=bezpiecze')
            ->assertOk()
            ->assertDontSeeText('Bezpieczeństwo konta')
            ->assertDontSeeText('Account security');
    }

    private function article(
        string $enTitle,
        string $enSlug,
        ?string $plTitle,
        ?string $plSlug,
        string $body,
        string $status = WikiArticleStatus::PUBLISHED->value,
    ): int {
        $id = DB::table('wiki_articles')->insertGetId([
            'content_type' => 'guide',
            'status' => $status,
            'is_featured' => false,
            'sort_order' => 0,
            'published_at' => $status === WikiArticleStatus::PUBLISHED->value ? now()->subMinute() : null,
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
            'source_markdown' => $body,
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
                'source_markdown' => $body,
                'created_at' => now(),
                'updated_at' => now(),
            ];
        }
        DB::table('wiki_article_translations')->insert($translations);

        return $id;
    }
}
