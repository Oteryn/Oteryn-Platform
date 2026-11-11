<?php

namespace App\Wiki\Infrastructure\Factories;

use App\Wiki\Infrastructure\Models\WikiCategoryTranslation;

final class WikiCategoryTranslationFactory
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
    public function create(array $attributes = []): WikiCategoryTranslation
    {
        self::$sequence++;
        $categoryId = $attributes['category_id'] ?? $this->state['category_id'] ?? WikiCategoryFactory::new()->create()->id;

        return WikiCategoryTranslation::query()->create(array_merge([
            'category_id' => $categoryId,
            'locale' => 'en',
            'name' => 'Wiki category '.self::$sequence,
            'slug' => 'wiki-category-'.self::$sequence,
            'description' => 'Factory category description.',
        ], $this->state, $attributes));
    }
}
