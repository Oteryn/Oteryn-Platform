<?php

namespace App\Cms;

use App\Cms\Editorial\EditorialContentType;
use App\Cms\Models\NewsPost;
use DateTimeInterface;
use Illuminate\Contracts\Pagination\LengthAwarePaginator;
use Illuminate\Database\Eloquent\Builder;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Support\Collection;
use InvalidArgumentException;

final class PublicNewsQuery
{
    /** @return LengthAwarePaginator<int, NewsPost> */
    public function published(int $perPage = 10, ?DateTimeInterface $readTime = null): LengthAwarePaginator
    {
        $readTime ??= now();

        return $this->visibleAt($readTime)
            ->orderByDesc('news_posts.published_at')
            ->orderByDesc('news_posts.id')
            ->paginate($perPage);
    }

    /** @return Collection<int, NewsPost> */
    public function latestPublished(int $limit = 3, ?DateTimeInterface $readTime = null): Collection
    {
        if ($limit < 1 || $limit > 10) {
            throw new InvalidArgumentException('Latest published news limit must be between 1 and 10.');
        }

        $readTime ??= now();

        return $this->visibleAt($readTime)
            ->orderByDesc('news_posts.published_at')
            ->orderByDesc('news_posts.id')
            ->limit($limit)
            ->get();
    }

    /** @return Collection<int, NewsPost> */
    public function searchPublished(
        string $search,
        int $page = 1,
        int $perPage = 5,
        ?DateTimeInterface $readTime = null,
    ): Collection {
        if ($page < 1 || $page > 100 || $perPage < 1 || $perPage > 10) {
            throw new InvalidArgumentException('Published news search pagination is outside bounds.');
        }

        $normalized = trim(preg_replace('/\\s+/u', ' ', $search) ?? $search);
        if ($normalized === '') {
            return collect();
        }

        $readTime ??= now();
        $needle = mb_strtolower($normalized);
        $escaped = str_replace(['!', '%', '_'], ['!!', '!%', '!_'], $needle);
        $prefix = $escaped.'%';
        $contains = '%'.$escaped.'%';
        $translated = app()->getLocale() === 'pl';
        $title = $translated ? 'public_news_translation.title' : 'news_posts.title';
        $body = $translated ? 'public_news_translation.body' : 'news_posts.body';

        return $this->visibleAt($readTime)
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
            ->orderByDesc('news_posts.published_at')
            ->orderBy('news_posts.id')
            ->offset(($page - 1) * $perPage)
            ->limit($perPage)
            ->get();
    }

    public function findPublishedBySlug(string $slug, ?DateTimeInterface $readTime = null): ?NewsPost
    {
        $readTime ??= now();

        return $this->visibleAt($readTime)
            ->where('news_posts.slug', $slug)
            ->first();
    }

    /** @return list<string> */
    public function publishedSlugs(?DateTimeInterface $readTime = null): array
    {
        $readTime ??= now();

        return array_values($this->visibleAt($readTime)
            ->orderBy('news_posts.slug')
            ->get(['news_posts.slug'])
            ->map(static fn (NewsPost $post): string => $post->slug)
            ->all());
    }

    /** @return Builder<NewsPost> */
    private function visibleAt(DateTimeInterface $readTime): Builder
    {
        $query = NewsPost::query()
            ->whereNotNull('news_posts.published_at')
            ->where('news_posts.published_at', '<=', $readTime);

        if (app()->getLocale() !== 'pl') {
            return $query;
        }

        $alias = 'public_news_translation';

        return $query
            ->join('editorial_translations as '.$alias, static function (JoinClause $join) use ($alias, $readTime): void {
                $join->on($alias.'.content_id', '=', 'news_posts.id')
                    ->where($alias.'.content_type', EditorialContentType::NewsPost->value)
                    ->where($alias.'.locale', 'pl')
                    ->whereNotNull($alias.'.title')
                    ->whereNotNull($alias.'.body')
                    ->whereNotNull($alias.'.published_at')
                    ->where($alias.'.published_at', '<=', $readTime)
                    ->whereColumn($alias.'.source_updated_at', '>=', 'news_posts.updated_at');
            })
            ->select('news_posts.*')
            ->addSelect([
                $alias.'.title as title',
                $alias.'.body as body',
            ]);
    }
}
