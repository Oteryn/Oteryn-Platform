<?php

namespace App\Wiki\Infrastructure\Factories;

use App\Wiki\Infrastructure\Models\WikiArticleTranslation;

final class WikiArticleTranslationFactory
{
    private static int $sequence = 0;

    /**
     * @param  array<string, mixed>  $state
     */
    private function __construct(private readonly array $state = []) {}

    public static function new(): self
    {
        return new self;
    }

    /**
     * @param  array<string, mixed>  $state
     */
    public function state(array $state): self
    {
        return new self(array_merge($this->state, $state));
    }

    /**
     * @param  array<string, mixed>  $attributes
     */
    public function create(array $attributes = []): WikiArticleTranslation
    {
        self::$sequence++;
        $articleId = $attributes['article_id'] ?? $this->state['article_id'] ?? WikiArticleFactory::new()->create()->id;

        return WikiArticleTranslation::query()->create(array_merge([
            'article_id' => $articleId,
            'locale' => 'en',
            'title' => 'Wiki article '.self::$sequence,
            'slug' => 'wiki-article-'.self::$sequence,
            'summary' => 'Factory summary.',
            'source_markdown' => '# Factory source',
        ], $this->state, $attributes));
    }
}
