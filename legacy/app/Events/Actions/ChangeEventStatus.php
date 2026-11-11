<?php

namespace App\Events\Actions;

use App\Audit\AdminAuditRecorder;
use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
use DateTimeInterface;
use DomainException;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;

final class ChangeEventStatus
{
    public function __construct(private readonly AdminAuditRecorder $audit) {}

    public function execute(
        Identity $actor,
        Event $event,
        string $status,
        int $expectedLockVersion,
        ?DateTimeInterface $transitionTime = null,
    ): Event {
        if (! in_array($status, Event::statuses(), true)) {
            throw new InvalidArgumentException('Unknown event publication state.');
        }

        $at = CarbonImmutable::instance($transitionTime ?? now())->utc();

        return DB::transaction(function () use ($actor, $event, $status, $expectedLockVersion, $at): Event {
            $current = Event::query()->lockForUpdate()->findOrFail($event->id);

            if ($current->lock_version !== $expectedLockVersion) {
                throw new DomainException('This event changed after the form was opened. Reload it before changing publication state.');
            }

            $this->assertTransition($current, $status, $at);

            $previousStatus = $current->status;
            $current->fill([
                'status' => $status,
                'updated_by' => $actor->id,
                'published_by' => $status === Event::STATUS_DRAFT ? null : $actor->id,
                'lock_version' => $current->lock_version + 1,
            ]);
            $current->save();

            $this->audit->record(
                $actor->id,
                'events.status_changed',
                'event',
                (string) $current->id,
                [
                    'from' => $previousStatus,
                    'to' => $current->status,
                    'lock_version' => $current->lock_version,
                ],
            );

            return $current;
        }, 3);
    }

    private function assertTransition(Event $event, string $status, CarbonImmutable $at): void
    {
        if ($status !== Event::STATUS_DRAFT && ! EventTranslation::query()
            ->where('event_id', $event->id)
            ->where('locale', 'en')
            ->exists()) {
            throw new DomainException('An English translation is required before publication.');
        }

        if ($status === Event::STATUS_SCHEDULED && ! $event->starts_at->isAfter($at)) {
            throw new DomainException('Scheduled events must start in the future.');
        }

        if (
            $status === Event::STATUS_ACTIVE
            && ($event->starts_at->isAfter($at) || ! $event->ends_at->isAfter($at))
        ) {
            throw new DomainException('Active events must include the current UTC time.');
        }

        if ($status === Event::STATUS_COMPLETED && $event->ends_at->isAfter($at)) {
            throw new DomainException('Completed events must have ended already.');
        }

        if ($status === Event::STATUS_CANCELLED && $event->status === Event::STATUS_DRAFT) {
            throw new DomainException('A draft event cannot be cancelled before it is published.');
        }
    }
}
