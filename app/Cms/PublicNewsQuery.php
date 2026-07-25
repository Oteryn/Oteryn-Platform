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

    public function findPublishedBySlug(string $slug, ?DateTimeInterface $readTime = null): ?NewsPost
    {
        $readTime ??= now();

        return $this->visibleAt($readTime)
            ->where('news_posts.slug', $slug)
            ->first();
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
