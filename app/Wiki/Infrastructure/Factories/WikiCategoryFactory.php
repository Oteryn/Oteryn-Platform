<?php

namespace App\Wiki\Infrastructure\Factories;

use App\Wiki\Infrastructure\Models\WikiCategory;

final class WikiCategoryFactory
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
    public function create(array $attributes = []): WikiCategory
    {
        self::$sequence++;

        return WikiCategory::query()->create(array_merge([
            'key' => 'category-'.self::$sequence,
            'sort_order' => 0,
            'visible' => true,
            'lock_version' => 1,
        ], $this->state, $attributes));
    }
}
