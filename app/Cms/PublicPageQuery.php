<?php

namespace App\Cms;

use App\Cms\Editorial\EditorialContentType;
use App\Cms\Models\ManagedPage;
use DateTimeInterface;
use Illuminate\Database\Query\JoinClause;

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
}
