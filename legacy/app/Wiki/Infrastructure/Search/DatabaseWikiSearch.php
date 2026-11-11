<?php

namespace App\Wiki\Infrastructure\Search;

use App\Wiki\Application\Search\InvalidWikiSearch;
use App\Wiki\Application\Search\WikiSearch;
use App\Wiki\Application\Search\WikiSearchError;
use App\Wiki\Application\Search\WikiSearchPage;
use App\Wiki\Application\Search\WikiSearchResult;
use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Domain\WikiContentRules;
use Illuminate\Database\Query\Builder;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use stdClass;
use UnexpectedValueException;

final class DatabaseWikiSearch implements WikiSearch
{
    private const MAX_QUERY_LENGTH = 80;

    private const MAX_PAGE = 1000;

    private const MAX_PER_PAGE = 24;

    public function search(string $locale, string $query, int $page, int $perPage = 12): WikiSearchPage
    {
        WikiContentRules::assertSupportedLocale($locale);

        $normalized = trim(preg_replace('/\s+/u', ' ', $query) ?? $query);
        if ($normalized === '') {
            return new WikiSearchPage($normalized, [], 1, $perPage, 0);
        }
        if (mb_strlen($normalized) < 2) {
            throw new InvalidWikiSearch(WikiSearchError::TooShort);
        }
        if (mb_strlen($normalized) > self::MAX_QUERY_LENGTH) {
            throw new InvalidWikiSearch(WikiSearchError::TooLong);
        }
        if ($page < 1 || $page > self::MAX_PAGE || $perPage < 1 || $perPage > self::MAX_PER_PAGE) {
            throw new InvalidWikiSearch(WikiSearchError::PageOutsideBounds);
        }

        $needle = mb_strtolower($normalized);
        $escaped = str_replace(['!', '%', '_'], ['!!', '!%', '!_'], $needle);
        $prefix = $escaped.'%';
        $contains = '%'.$escaped.'%';
        $base = $this->publishedSearchBoundary($locale)
            ->where(function (Builder $matches) use ($contains): void {
                $matches
                    ->whereRaw("LOWER(wt.title) LIKE ? ESCAPE '!'", [$contains])
                    ->orWhereRaw("LOWER(wt.summary) LIKE ? ESCAPE '!'", [$contains])
                    ->orWhereRaw("LOWER(wt.source_markdown) LIKE ? ESCAPE '!'", [$contains]);
            });

        $total = (clone $base)->count('a.id');
        $rows = $base
            ->select(['a.id', 'wt.title', 'wt.slug', 'wt.summary', 'a.published_at'])
            ->orderByRaw(
                "CASE
                    WHEN LOWER(wt.title) = ? THEN 0
                    WHEN LOWER(wt.title) LIKE ? ESCAPE '!' THEN 1
                    WHEN LOWER(wt.title) LIKE ? ESCAPE '!' THEN 2
                    WHEN LOWER(wt.summary) LIKE ? ESCAPE '!' THEN 3
                    ELSE 4
                END",
                [$needle, $prefix, $contains, $contains],
            )
            ->orderBy('wt.title')
            ->orderBy('a.id')
            ->offset(($page - 1) * $perPage)
            ->limit($perPage)
            ->get();

        return new WikiSearchPage(
            $normalized,
            array_values($rows->map(fn (stdClass $row): WikiSearchResult => new WikiSearchResult(
                $this->integer($row->id),
                $this->string($row->title),
                $this->string($row->slug),
                $this->string($row->summary),
                Carbon::parse($this->string($row->published_at)),
            ))->all()),
            $page,
            $perPage,
            $total,
        );
    }

    private function publishedSearchBoundary(string $locale): Builder
    {
        $query = DB::table('wiki_articles as a')
            ->join('wiki_article_translations as wt', function (JoinClause $join) use ($locale): void {
                $join->on('wt.article_id', '=', 'a.id')
                    ->where('wt.locale', $locale);
            });

        if ($locale !== 'en') {
            $query->join('wiki_article_translations as source_wt', function (JoinClause $join): void {
                $join->on('source_wt.article_id', '=', 'a.id')
                    ->where('source_wt.locale', 'en');
            })->whereColumn('wt.updated_at', '>=', 'source_wt.updated_at');
        }

        return $query
            ->where('a.status', WikiArticleStatus::PUBLISHED->value)
            ->whereNotNull('a.published_at')
            ->where('a.published_at', '<=', now());
    }

    private function string(mixed $value): string
    {
        if (! is_string($value) && ! is_int($value) && ! is_float($value)) {
            throw new UnexpectedValueException('Expected a scalar Wiki search value.');
        }

        return (string) $value;
    }

    private function integer(mixed $value): int
    {
        if (is_int($value)) {
            return $value;
        }
        if (! is_string($value) || preg_match('/\A\d+\z/D', $value) !== 1) {
            throw new UnexpectedValueException('Expected an integer Wiki search value.');
        }

        return (int) $value;
    }
}
