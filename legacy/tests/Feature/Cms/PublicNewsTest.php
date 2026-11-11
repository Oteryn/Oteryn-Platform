<?php

namespace Tests\Feature\Cms;

use App\Cms\Models\NewsPost;
use Illuminate\Database\QueryException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;

final class PublicNewsTest extends TestCase
{
    use RefreshDatabase;

    public function test_news_posts_schema_has_required_platform_owned_columns(): void
    {
        self::assertTrue(Schema::hasColumns('news_posts', [
            'id',
            'slug',
            'title',
            'body',
            'published_at',
            'created_at',
            'updated_at',
        ]));
    }

    public function test_news_slug_is_unique(): void
    {
        $this->createNews('same-slug', 'First', now()->subHour());

        $this->expectException(QueryException::class);

        $this->createNews('same-slug', 'Second', now()->subHour());
    }

    public function test_index_shows_only_published_posts_in_deterministic_order(): void
    {
        $this->createNews('older', 'Older published', now()->subHours(2));
        $this->createNews('newer', 'Newer published', now()->subHour());
        $this->createNews('draft', 'Draft hidden', null);
        $this->createNews('scheduled', 'Scheduled hidden', now()->addHour());

        $this->get(route('news.index'))
            ->assertOk()
            ->assertSeeInOrder(['Newer published', 'Older published'])
            ->assertDontSee('Draft hidden')
            ->assertDontSee('Scheduled hidden');
    }

    public function test_news_index_is_paginated_to_ten_posts(): void
    {
        $publishedAt = now()->subHour();

        for ($index = 1; $index <= 11; $index++) {
            $this->createNews(
                "entry-{$index}",
                sprintf('Entry %03d', $index),
                $publishedAt,
            );
        }

        $this->get(route('news.index'))
            ->assertOk()
            ->assertSee('Entry 011')
            ->assertDontSee('Entry 001')
            ->assertSee('Page 1 of 2');

        $this->get(route('news.index', ['page' => 2]))
            ->assertOk()
            ->assertSee('Entry 001')
            ->assertDontSee('Entry 011')
            ->assertSee('Page 2 of 2');
    }

    public function test_detail_returns_only_published_posts(): void
    {
        $this->createNews('published', 'Published post', now()->subHour(), 'Published body');
        $this->createNews('draft', 'Draft post', null, 'Draft body');
        $this->createNews('scheduled', 'Scheduled post', now()->addHour(), 'Scheduled body');

        $this->get(route('news.show', ['slug' => 'published']))
            ->assertOk()
            ->assertSee('Published post')
            ->assertSee('Published body');

        $this->get(route('news.show', ['slug' => 'draft']))->assertNotFound();
        $this->get(route('news.show', ['slug' => 'scheduled']))->assertNotFound();
    }

    public function test_news_output_escapes_title_and_plain_text_body(): void
    {
        $this->createNews(
            'unsafe',
            '<script>alert("title")</script>',
            now()->subHour(),
            '<img src=x onerror=alert("body")>',
        );

        $this->get(route('news.show', ['slug' => 'unsafe']))
            ->assertOk()
            ->assertSee('&lt;script&gt;alert(&quot;title&quot;)&lt;/script&gt;', false)
            ->assertSee('&lt;img src=x onerror=alert(&quot;body&quot;)&gt;', false)
            ->assertDontSee('<script>', false)
            ->assertDontSee('<img src=x', false);
    }

    private function createNews(
        string $slug,
        string $title,
        ?Carbon $publishedAt,
        string $body = 'Body',
    ): NewsPost {
        return NewsPost::query()->create([
            'slug' => $slug,
            'title' => $title,
            'body' => $body,
            'published_at' => $publishedAt,
        ]);
    }
}
