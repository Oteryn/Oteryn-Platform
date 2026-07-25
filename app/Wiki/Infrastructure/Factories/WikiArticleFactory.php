<?php

namespace App\Wiki\Infrastructure\Factories;

use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Infrastructure\Models\WikiArticle;

final class WikiArticleFactory
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
    public function create(array $attributes = []): WikiArticle
    {
        return WikiArticle::query()->create(array_merge([
            'content_type' => 'guide',
            'status' => WikiArticleStatus::DRAFT,
            'is_featured' => false,
            'sort_order' => 0,
            'lock_version' => 1,
        ], $this->state, $attributes));
    }
}
