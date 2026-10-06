<?php

namespace App\Announcements\Queries;

use App\Announcements\Models\SiteAnnouncement;
use App\Cms\Editorial\EditorialContentType;
use DateTimeInterface;
use Illuminate\Database\Eloquent\Builder;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Support\Collection;
use InvalidArgumentException;

final class ActiveAnnouncementQuery
{
    /**
     * Start is inclusive and end is exclusive.
     *
     * @return Collection<int, SiteAnnouncement>
     */
    public function active(int $limit = 5, ?DateTimeInterface $readTime = null): Collection
    {
        if ($limit < 1 || $limit > 10) {
            throw new InvalidArgumentException('Active announcement limit must be between 1 and 10.');
        }

        $readTime ??= now();
        $query = SiteAnnouncement::query()
            ->where('site_announcements.publication_state', SiteAnnouncement::STATE_PUBLISHED)
            ->where('site_announcements.starts_at', '<=', $readTime)
            ->where(
                /** @param Builder<SiteAnnouncement> $query */
                function (Builder $query) use ($readTime): void {
                    $query
                        ->whereNull('site_announcements.ends_at')
                        ->orWhere('site_announcements.ends_at', '>', $readTime);
                },
            );

        if (app()->getLocale() === 'pl') {
            $alias = 'public_announcement_translation';
            $query
                ->join('editorial_translations as '.$alias, static function (JoinClause $join) use ($alias, $readTime): void {
                    $join->on($alias.'.content_id', '=', 'site_announcements.id')
                        ->where($alias.'.content_type', EditorialContentType::SiteAnnouncement->value)
                        ->where($alias.'.locale', 'pl')
                        ->whereNotNull($alias.'.title')
                        ->whereNotNull($alias.'.body')
                        ->whereNotNull($alias.'.published_at')
                        ->where($alias.'.published_at', '<=', $readTime)
                        ->whereColumn($alias.'.source_updated_at', '>=', 'site_announcements.updated_at');
                })
                ->select('site_announcements.*')
                ->addSelect([
                    $alias.'.title as title',
                    $alias.'.body as body',
                    $alias.'.action_label as action_label',
                ]);
        }

        return $query
            ->orderByDesc('site_announcements.severity')
            ->orderByDesc('site_announcements.starts_at')
            ->orderByDesc('site_announcements.id')
            ->limit($limit)
            ->get();
    }

    /**
     * Searchable announcements require a local first-party action path so search never upgrades
     * an arbitrary external CTA into a canonical content URL.
     *
     * @return Collection<int, SiteAnnouncement>
     */
    public function searchActive(
        string $search,
        int $page = 1,
        int $perPage = 5,
        ?DateTimeInterface $readTime = null,
    ): Collection {
        if ($page < 1 || $page > 100 || $perPage < 1 || $perPage > 10) {
            throw new InvalidArgumentException('Announcement search pagination is outside bounds.');
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

        $query = SiteAnnouncement::query()
            ->where('site_announcements.publication_state', SiteAnnouncement::STATE_PUBLISHED)
            ->where('site_announcements.starts_at', '<=', $readTime)
            ->where(function (Builder $query) use ($readTime): void {
                $query
                    ->whereNull('site_announcements.ends_at')
                    ->orWhere('site_announcements.ends_at', '>', $readTime);
            })
            ->where('site_announcements.action_url', 'like', '/%')
            ->where('site_announcements.action_url', 'not like', '//%');

        $title = 'site_announcements.title';
        $body = 'site_announcements.body';
        if (app()->getLocale() === 'pl') {
            $alias = 'public_announcement_translation';
            $query
                ->join('editorial_translations as '.$alias, static function (JoinClause $join) use ($alias, $readTime): void {
                    $join->on($alias.'.content_id', '=', 'site_announcements.id')
                        ->where($alias.'.content_type', EditorialContentType::SiteAnnouncement->value)
                        ->where($alias.'.locale', 'pl')
                        ->whereNotNull($alias.'.title')
                        ->whereNotNull($alias.'.body')
                        ->whereNotNull($alias.'.published_at')
                        ->where($alias.'.published_at', '<=', $readTime)
                        ->whereColumn($alias.'.source_updated_at', '>=', 'site_announcements.updated_at');
                })
                ->select('site_announcements.*')
                ->addSelect([
                    $alias.'.title as title',
                    $alias.'.body as body',
                    $alias.'.action_label as action_label',
                ]);
            $title = $alias.'.title';
            $body = $alias.'.body';
        }

        return $query
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
            ->orderByDesc('site_announcements.severity')
            ->orderByDesc('site_announcements.starts_at')
            ->orderBy('site_announcements.id')
            ->offset(($page - 1) * $perPage)
            ->limit($perPage)
            ->get();
    }
}
