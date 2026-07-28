<?php

namespace App\GameCatalog\Http;

use App\GameCatalog\Queries\PublicCatalogQuery;
use Illuminate\Contracts\View\View;
use Illuminate\Http\Request;
use Illuminate\Pagination\LengthAwarePaginator;

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
        $context = $this->catalog->context();

        return view('game-catalog.items.index', [
            'context' => $context,
            'items' => $context === null
                ? $this->emptyPaginator($request, $filters['per_page'])
                : $this->catalog->items($filters),
            'categories' => $context === null ? [] : $this->catalog->itemCategories(),
            'filters' => $filters,
        ]);
    }

    public function item(string $locale, string $slug): View
    {
        $context = $this->catalog->context();
        abort_if($context === null, 404);
        $item = $this->catalog->item($slug);
        abort_if($item === null, 404);

        return view('game-catalog.items.show', [
            'context' => $context,
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
        $context = $this->catalog->context();

        return view('game-catalog.creatures.index', [
            'context' => $context,
            'creatures' => $context === null
                ? $this->emptyPaginator($request, $filters['per_page'])
                : $this->catalog->creatures($filters),
            'bestiaryClasses' => $context === null ? [] : $this->catalog->bestiaryClasses(),
            'filters' => $filters,
        ]);
    }

    public function creature(string $locale, string $slug): View
    {
        $context = $this->catalog->context();
        abort_if($context === null, 404);
        $creature = $this->catalog->creature($slug);
        abort_if($creature === null, 404);

        return view('game-catalog.creatures.show', [
            'context' => $context,
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

    private function emptyPaginator(Request $request, int $perPage): LengthAwarePaginator
    {
        return new LengthAwarePaginator([], 0, $perPage, max(1, $request->integer('page', 1)), [
            'path' => $request->url(),
            'query' => $request->query(),
        ]);
    }
}
