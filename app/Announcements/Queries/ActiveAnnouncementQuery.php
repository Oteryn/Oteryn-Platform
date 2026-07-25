<?php

namespace App\Announcements\Queries;

use App\Announcements\Models\SiteAnnouncement;
use DateTimeInterface;
use Illuminate\Database\Eloquent\Builder;
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

        return SiteAnnouncement::query()
            ->where('publication_state', SiteAnnouncement::STATE_PUBLISHED)
            ->where('starts_at', '<=', $readTime)
            ->where(
                /** @param Builder<SiteAnnouncement> $query */
                function (Builder $query) use ($readTime): void {
                    $query
                        ->whereNull('ends_at')
                        ->orWhere('ends_at', '>', $readTime);
                },
            )
            ->orderByDesc('severity')
            ->orderByDesc('starts_at')
            ->orderByDesc('id')
            ->limit($limit)
            ->get();
    }
}
