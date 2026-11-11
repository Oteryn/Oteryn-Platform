<?php

namespace App\Support\Notifications;

use App\Identity\Models\Identity;
use App\Notifications\Support\SupportLifecycleNotification;
use App\Support\Models\SupportNotificationDelivery;
use Illuminate\Support\Facades\DB;
use Throwable;

final class SupportNotificationDeliveryService
{
    public const TICKET_REPLY = 'ticket_reply';

    public const TICKET_STATUS = 'ticket_status';

    public const REPORT_OUTCOME = 'report_outcome';

    public const ENFORCEMENT_CREATED = 'enforcement_created';

    public const ENFORCEMENT_UPDATED = 'enforcement_updated';

    public const APPEAL_OUTCOME = 'appeal_outcome';

    public function queue(
        Identity $identity,
        string $eventKey,
        string $relatedType,
        string $relatedId,
        string $locale = 'en',
    ): SupportNotificationDelivery {
        $normalizedLocale = in_array($locale, ['en', 'pl'], true) ? $locale : 'en';

        $delivery = SupportNotificationDelivery::query()->updateOrCreate(
            [
                'identity_id' => $identity->id,
                'event_key' => $eventKey,
                'related_type' => $relatedType,
                'related_id' => $relatedId,
            ],
            [
                'locale' => $normalizedLocale,
                'status' => SupportNotificationDelivery::STATUS_PENDING,
                'last_error_code' => null,
                'failed_at' => null,
            ],
        );

        DB::afterCommit(function () use ($delivery): void {
            $this->deliver($delivery->id);
        });

        return $delivery;
    }

    public function deliver(int $deliveryId): void
    {
        $delivery = SupportNotificationDelivery::query()->with('identity')->find($deliveryId);

        if (! $delivery instanceof SupportNotificationDelivery || $delivery->status === SupportNotificationDelivery::STATUS_SENT) {
            return;
        }

        $identity = $delivery->identity;
        if (! $identity instanceof Identity || $identity->isTerminated()) {
            $delivery->forceFill([
                'status' => SupportNotificationDelivery::STATUS_FAILED,
                'attempts' => $delivery->attempts + 1,
                'last_error_code' => 'notifiable_unavailable',
                'failed_at' => now(),
            ])->save();

            return;
        }

        try {
            $identity->notify(new SupportLifecycleNotification(
                $delivery->event_key,
                $delivery->related_type,
                $delivery->related_id,
                $delivery->locale,
            ));

            $delivery->forceFill([
                'status' => SupportNotificationDelivery::STATUS_SENT,
                'attempts' => $delivery->attempts + 1,
                'last_error_code' => null,
                'sent_at' => now(),
                'failed_at' => null,
            ])->save();
        } catch (Throwable $exception) {
            $delivery->forceFill([
                'status' => SupportNotificationDelivery::STATUS_FAILED,
                'attempts' => $delivery->attempts + 1,
                'last_error_code' => substr(class_basename($exception), 0, 64),
                'failed_at' => now(),
            ])->save();
        }
    }
}
