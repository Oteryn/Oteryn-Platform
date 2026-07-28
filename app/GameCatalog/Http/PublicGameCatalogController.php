<?php

namespace App\GameCatalog\Http;

use App\GameCatalog\Queries\PublicCatalogQuery;
use Illuminate\Contracts\View\View;
use Illuminate\Http\Request;

final readonly class PublicGameCatalogController
{
    public function __construct(private PublicCatalogQuery $catalog) {}

    public function index(): View
    {
        $context = $this->catalog->context();
        $categories = $context === null ? [] : $this->catalog->itemCategories();
        $bestiaryClasses = $context === null ? [] : $this->catalog->bestiaryClasses();

        return view('game-catalog.index', compact('context', 'categories', 'bestiaryClasses'));
    }

    public function items(Request $request): View
    {
        $filters = [
            'category' => $this->boundedToken($request->query('category'), 80),
            'weapon_type' => $this->boundedToken($request->query('weapon_type'), 40),
            'q' => $this->boundedSearch($request->query('q')),
            'per_page' => $this->perPage($request),
        ];

        return view('game-catalog.items.index', [
            'context' => $this->catalog->context(),
            'items' => $this->catalog->items($filters),
            'categories' => $this->catalog->itemCategories(),
            'filters' => $filters,
        ]);
    }

    public function item(string $locale, string $slug): View
    {
        $item = $this->catalog->item($slug);
        abort_if($item === null, 404);

        return view('game-catalog.items.show', [
            'context' => $this->catalog->context(),
            'item' => $item,
            'lootSources' => $this->catalog->itemLootSources((int) $item->entity_id),
        ]);
    }

    public function creatures(Request $request): View
    {
        $boss = $request->query('boss');
        $filters = [
            'q' => $this->boundedSearch($request->query('q')),
            'boss' => $boss === '1' ? true : ($boss === '0' ? false : null),
            'bestiary_class' => $this->boundedToken($request->query('bestiary_class'), 120),
            'per_page' => $this->perPage($request),
        ];

        return view('game-catalog.creatures.index', [
            'context' => $this->catalog->context(),
            'creatures' => $this->catalog->creatures($filters),
            'bestiaryClasses' => $this->catalog->bestiaryClasses(),
            'filters' => $filters,
        ]);
    }

    public function creature(string $locale, string $slug): View
    {
        $creature = $this->catalog->creature($slug);
        abort_if($creature === null, 404);

        return view('game-catalog.creatures.show', [
            'context' => $this->catalog->context(),
            'creature' => $creature,
            'loot' => $this->catalog->creatureLoot((int) $creature->entity_id),
        ]);
    }

    private function boundedSearch(mixed $value): ?string
    {
        if (! is_string($value)) {
            return null;
        }
        $value = trim($value);

        return $value !== '' && mb_strlen($value) <= 100 ? $value : null;
    }

    private function boundedToken(mixed $value, int $maximum): ?string
    {
        if (! is_string($value) || mb_strlen($value) > $maximum || preg_match('/^[a-z0-9][a-z0-9._-]*$/D', $value) !== 1) {
            return null;
        }

        return $value;
    }

    private function perPage(Request $request): int
    {
        $default = config('game-catalog.pagination.default', 24);
        $maximum = config('game-catalog.pagination.maximum', 100);
        $value = filter_var($request->query('per_page'), FILTER_VALIDATE_INT, ['options' => ['min_range' => 1]]);

        if (! is_int($default) || ! is_int($maximum) || ! is_int($value) || $value > $maximum) {
            return is_int($default) ? $default : 24;
        }

        return $value;
    }
}
