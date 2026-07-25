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
}
