<?php

namespace App\Cms;

use App\Cms\Editorial\EditorialContentType;
use App\Cms\Models\ManagedPage;
use DateTimeInterface;
use Illuminate\Database\Eloquent\Builder;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Support\Collection;
use InvalidArgumentException;

final class PublicPageQuery
{
    public function findPublishedBySlug(string $slug, ?DateTimeInterface $readTime = null): ?ManagedPage
    {
        $readTime ??= now();
        $query = ManagedPage::query()
            ->where('managed_pages.slug', $slug)
            ->whereNotNull('managed_pages.published_at')
            ->where('managed_pages.published_at', '<=', $readTime);

        if (app()->getLocale() !== 'pl') {
            return $query->first();
        }

        $alias = 'public_page_translation';

        return $query
            ->join('editorial_translations as '.$alias, static function (JoinClause $join) use ($alias, $readTime): void {
                $join->on($alias.'.content_id', '=', 'managed_pages.id')
                    ->where($alias.'.content_type', EditorialContentType::ManagedPage->value)
                    ->where($alias.'.locale', 'pl')
                    ->whereNotNull($alias.'.title')
                    ->whereNotNull($alias.'.body')
                    ->whereNotNull($alias.'.published_at')
                    ->where($alias.'.published_at', '<=', $readTime)
                    ->whereColumn($alias.'.source_updated_at', '>=', 'managed_pages.updated_at');
            })
            ->select('managed_pages.*')
            ->addSelect([
                $alias.'.title as title',
                $alias.'.body as body',
            ])
            ->first();
    }

    /** @return Collection<int, ManagedPage> */
    public function searchPublished(
        string $search,
        string $locale,
        int $offset = 0,
        int $limit = 5,
        ?DateTimeInterface $readTime = null,
    ): Collection {
        if (! in_array($locale, ['en', 'pl'], true)) {
            throw new InvalidArgumentException('Unsupported search locale.');
        }
        if ($offset < 0 || $offset > 1000 || $limit < 1 || $limit > 10) {
            throw new InvalidArgumentException('Published page search pagination is outside bounds.');
        }

        $normalized = trim(preg_replace('/\\s+/u', ' ', $search) ?? $search);
        if ($normalized === '') {
            return new Collection;
        }

        $readTime ??= now();
        $needle = mb_strtolower($normalized);
        $escaped = str_replace(['!', '%', '_'], ['!!', '!%', '!_'], $needle);
        $prefix = $escaped.'%';
        $contains = '%'.$escaped.'%';
        $translated = $locale === 'pl';
        $title = $translated ? 'public_page_translation.title' : 'managed_pages.title';
        $body = $translated ? 'public_page_translation.body' : 'managed_pages.body';

        /** @var Collection<int, ManagedPage> $rows */
        $rows = $this->visibleAt($readTime, $locale)
            ->where(function (Builder $matches) use ($title, $body, $contains): void {
                $matches
                    ->whereRaw("LOWER({$title}) LIKE ? ESCAPE '!'", [$contains])
                    ->orWhereRaw("LOWER({$body}) LIKE ? ESCAPE '!'", [$contains]);
            })
            ->orderByRaw(
                "CASE
                    WHEN LOWER({$title}) = ? THEN 0
                    WHEN LOWER({$title}) LIKE ? ESCAPE '!' THEN 1
                    WHEN LOWER({$title}) LIKE ? ESCAPE '!' THEN 2
                    ELSE 3
                END",
                [$needle, $prefix, $contains],
            )
            ->orderBy('managed_pages.slug')
            ->offset($offset)
            ->limit($limit)
            ->get();

        return $rows;
    }

    /** @return list<string> */
    public function publishedSlugs(?DateTimeInterface $readTime = null): array
    {
        $readTime ??= now();

        return array_values($this->visibleAt($readTime)
            ->orderBy('managed_pages.slug')
            ->get(['managed_pages.slug'])
            ->map(static fn (ManagedPage $page): string => $page->slug)
            ->all());
    }

    /** @return Builder<ManagedPage> */
    private function visibleAt(DateTimeInterface $readTime, ?string $locale = null): Builder
    {
        $query = ManagedPage::query()
            ->whereNotNull('managed_pages.published_at')
            ->where('managed_pages.published_at', '<=', $readTime);

        if (($locale ?? app()->getLocale()) !== 'pl') {
            return $query;
        }

        $alias = 'public_page_translation';

        return $query
            ->join('editorial_translations as '.$alias, static function (JoinClause $join) use ($alias, $readTime): void {
                $join->on($alias.'.content_id', '=', 'managed_pages.id')
                    ->where($alias.'.content_type', EditorialContentType::ManagedPage->value)
                    ->where($alias.'.locale', 'pl')
                    ->whereNotNull($alias.'.title')
                    ->whereNotNull($alias.'.body')
                    ->whereNotNull($alias.'.published_at')
                    ->where($alias.'.published_at', '<=', $readTime)
                    ->whereColumn($alias.'.source_updated_at', '>=', 'managed_pages.updated_at');
            })
            ->select('managed_pages.*')
            ->addSelect([
                $alias.'.title as title',
                $alias.'.body as body',
            ]);
    }
}
