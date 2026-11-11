<?php

namespace App\Announcements\Actions;

use App\Announcements\Links\AnnouncementActionLink;
use App\Announcements\Models\SiteAnnouncement;
use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
use DateTimeInterface;
use DomainException;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;

final class SaveAnnouncement
{
    public function __construct(private readonly AdminAuditRecorder $audit) {}

    public function execute(
        Identity $actor,
        ?SiteAnnouncement $announcement,
        string $title,
        string $body,
        string $severity,
        DateTimeInterface $startsAt,
        ?DateTimeInterface $endsAt,
        string $publicationState,
        ?string $actionLabel,
        ?string $actionUrl,
        ?int $expectedLockVersion,
    ): SiteAnnouncement {
        $normalizedStartsAt = CarbonImmutable::instance($startsAt)->utc();
        $normalizedEndsAt = $endsAt === null ? null : CarbonImmutable::instance($endsAt)->utc();
        $normalizedActionUrl = AnnouncementActionLink::normalize($actionUrl);
        $normalizedActionLabel = $normalizedActionUrl === null ? null : trim((string) $actionLabel);

        $this->assertInput(
            $severity,
            $publicationState,
            $normalizedStartsAt,
            $normalizedEndsAt,
            $normalizedActionUrl,
            $normalizedActionLabel,
        );

        return DB::transaction(function () use (
            $actor,
            $announcement,
            $title,
            $body,
            $severity,
            $normalizedStartsAt,
            $normalizedEndsAt,
            $publicationState,
            $normalizedActionLabel,
            $normalizedActionUrl,
            $expectedLockVersion,
        ): SiteAnnouncement {
            $created = $announcement === null;

            if ($created) {
                $current = new SiteAnnouncement;
                $current->created_by = $actor->id;
                $current->lock_version = 1;
            } else {
                $current = SiteAnnouncement::query()
                    ->lockForUpdate()
                    ->findOrFail($announcement->id);

                if ($expectedLockVersion === null || $current->lock_version !== $expectedLockVersion) {
                    throw new DomainException('This announcement changed after the form was opened. Reload it before saving.');
                }

                $current->lock_version = $current->lock_version + 1;
            }

            $current->fill([
                'title' => $title,
                'body' => $body,
                'severity' => $severity,
                'starts_at' => $normalizedStartsAt,
                'ends_at' => $normalizedEndsAt,
                'publication_state' => $publicationState,
                'action_label' => $normalizedActionLabel,
                'action_url' => $normalizedActionUrl,
                'updated_by' => $actor->id,
                'published_by' => $publicationState === SiteAnnouncement::STATE_PUBLISHED ? $actor->id : null,
            ]);
            $current->save();

            $this->audit->record(
                $actor->id,
                $created ? 'portal.announcement_created' : 'portal.announcement_updated',
                'site_announcement',
                (string) $current->id,
                [
                    'severity' => $current->severity,
                    'publication_state' => $current->publication_state,
                    'starts_at' => $current->starts_at->toIso8601String(),
                    'ends_at' => $current->ends_at?->toIso8601String(),
                    'has_action' => $current->action_url !== null,
                    'lock_version' => $current->lock_version,
                ],
            );

            return $current;
        }, 3);
    }

    private function assertInput(
        string $severity,
        string $publicationState,
        CarbonImmutable $startsAt,
        ?CarbonImmutable $endsAt,
        ?string $actionUrl,
        ?string $actionLabel,
    ): void {
        if (! in_array($severity, SiteAnnouncement::severities(), true)) {
            throw new InvalidArgumentException('Unknown announcement severity.');
        }

        if (! in_array($publicationState, SiteAnnouncement::publicationStates(), true)) {
            throw new InvalidArgumentException('Unknown announcement publication state.');
        }

        if ($endsAt !== null && ! $endsAt->isAfter($startsAt)) {
            throw new InvalidArgumentException('The announcement end must be after its start.');
        }

        if ($actionUrl !== null && ($actionLabel === null || $actionLabel === '')) {
            throw new InvalidArgumentException('An action label is required when an action link is present.');
        }
    }
}
