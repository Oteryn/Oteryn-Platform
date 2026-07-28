<?php

namespace App\GameCatalog\Http\Public;

use App\GameCatalog\Queries\Public\DatabasePublicCatalogQuery;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use Illuminate\Http\Response as HttpResponse;
use Illuminate\View\View;
use Symfony\Component\HttpFoundation\Response;

final readonly class PublicGameCatalogController
{
    public function __construct(private DatabasePublicCatalogQuery $catalog) {}

    public function index(): View|HttpResponse
    {
        try {
            return view('game-catalog.index', [
                'catalog' => $this->catalog->summary(),
            ]);
        } catch (QueryException $exception) {
            return $this->unavailable($exception);
        }
    }

    public function items(Request $request): View|HttpResponse
    {
        try {
            return view('game-catalog.items.index', [
                'catalog' => $this->catalog->items(
                    locale: $this->locale($request),
                    query: $this->queryString($request, 'q', 80),
                    category: $this->optionalToken($request, 'category', 80),
                    weaponType: $this->optionalToken($request, 'weapon_type', 40),
                    page: $this->page($request),
                ),
            ]);
        } catch (QueryException $exception) {
            return $this->unavailable($exception);
        }
    }

    public function item(Request $request, string $slug): View|HttpResponse
    {
        try {
            $item = $this->catalog->item($this->locale($request), $slug);
            if ($item === null) {
                abort(Response::HTTP_NOT_FOUND);
            }

            return view('game-catalog.items.show', ['item' => $item]);
        } catch (QueryException $exception) {
            return $this->unavailable($exception);
        }
    }

    public function creatures(Request $request): View|HttpResponse
    {
        try {
            return view('game-catalog.creatures.index', [
                'catalog' => $this->catalog->creatures(
                    locale: $this->locale($request),
                    query: $this->queryString($request, 'q', 80),
                    bestiaryClass: $this->optionalToken($request, 'bestiary_class', 120),
                    bossOnly: $request->boolean('boss'),
                    page: $this->page($request),
                ),
            ]);
        } catch (QueryException $exception) {
            return $this->unavailable($exception);
        }
    }

    public function creature(Request $request, string $slug): View|HttpResponse
    {
        try {
            $creature = $this->catalog->creature($this->locale($request), $slug);
            if ($creature === null) {
                abort(Response::HTTP_NOT_FOUND);
            }

            return view('game-catalog.creatures.show', ['creature' => $creature]);
        } catch (QueryException $exception) {
            return $this->unavailable($exception);
        }
    }

    private function locale(Request $request): string
    {
        $locale = $request->route('locale', app()->getLocale());

        return is_string($locale) && in_array($locale, ['en', 'pl'], true)
            ? $locale
            : app()->getLocale();
    }

    private function queryString(Request $request, string $key, int $maximumLength): string
    {
        $value = $request->query($key, '');
        if (! is_string($value)) {
            abort(Response::HTTP_UNPROCESSABLE_ENTITY);
        }

        $value = trim($value);
        if (mb_strlen($value, 'UTF-8') > $maximumLength) {
            abort(Response::HTTP_UNPROCESSABLE_ENTITY);
        }

        return $value;
    }

    private function optionalToken(Request $request, string $key, int $maximumLength): ?string
    {
        $value = $request->query($key);
        if ($value === null || $value === '') {
            return null;
        }
        if (! is_string($value) || mb_strlen($value, 'UTF-8') > $maximumLength) {
            abort(Response::HTTP_UNPROCESSABLE_ENTITY);
        }
        if (preg_match('/^[\pL\pN _.-]+$/uD', $value) !== 1) {
            abort(Response::HTTP_UNPROCESSABLE_ENTITY);
        }

        return $value;
    }

    private function page(Request $request): int
    {
        $page = filter_var($request->query('page', 1), FILTER_VALIDATE_INT);

        return is_int($page) && $page >= 1 && $page <= 10_000 ? $page : 1;
    }

    private function unavailable(QueryException $exception): HttpResponse
    {
        report($exception);

        return response()->view('game-catalog.unavailable', [], Response::HTTP_SERVICE_UNAVAILABLE);
    }
}
