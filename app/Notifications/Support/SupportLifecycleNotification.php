<?php

namespace App\Notifications\Support;

use Illuminate\Bus\Queueable;
use Illuminate\Notifications\Messages\MailMessage;
use Illuminate\Notifications\Notification;
use Illuminate\Support\Facades\Lang;
use InvalidArgumentException;

final class SupportLifecycleNotification extends Notification
{
    use Queueable;

    public function __construct(
        private readonly string $eventKey,
        private readonly string $relatedType,
        private readonly string $relatedId,
        private readonly string $notificationLocale = 'en',
    ) {}

    /** @return list<string> */
    public function via(object $notifiable): array
    {
        return ['mail'];
    }

    public function toMail(object $notifiable): MailMessage
    {
        $locale = in_array($this->notificationLocale, ['en', 'pl'], true)
            ? $this->notificationLocale
            : 'en';

        return (new MailMessage)
            ->subject((string) Lang::get("support.mail.{$this->eventKey}.subject", [], $locale))
            ->line((string) Lang::get("support.mail.{$this->eventKey}.intro", [], $locale))
            ->action(
                (string) Lang::get('support.mail.open_action', [], $locale),
                $this->url($locale),
            )
            ->line((string) Lang::get('support.mail.outro', [], $locale));
    }

    private function url(string $locale): string
    {
        return match ($this->relatedType) {
            'ticket' => route('support.tickets.show', ['supportTicket' => $this->relatedId, 'locale' => $locale]),
            'report' => route('support.reports.show', ['playerReport' => $this->relatedId, 'locale' => $locale]),
            'enforcement' => route('support.enforcement.show', ['enforcementRecord' => $this->relatedId, 'locale' => $locale]),
            default => throw new InvalidArgumentException('Unsupported support notification relation.'),
        };
    }
}
