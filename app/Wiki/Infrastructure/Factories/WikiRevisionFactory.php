<?php

namespace App\Wiki\Infrastructure\Factories;

use App\Wiki\Infrastructure\Models\WikiRevision;

final class WikiRevisionFactory
{
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
    public function create(array $attributes = []): WikiRevision
    {
        $articleId = $attributes['article_id'] ?? $this->state['article_id'] ?? WikiArticleFactory::new()->create()->id;

        return WikiRevision::query()->create(array_merge([
            'article_id' => $articleId,
            'locale' => 'en',
            'revision_number' => 1,
            'article_version' => 1,
            'title' => 'Factory revision',
            'slug' => 'factory-revision',
            'summary' => 'Factory revision summary.',
            'source_markdown' => '# Factory revision source',
        ], $this->state, $attributes));
    }
}
