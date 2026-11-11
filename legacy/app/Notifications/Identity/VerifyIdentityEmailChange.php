<?php

namespace App\Notifications\Identity;

use Illuminate\Bus\Queueable;
use Illuminate\Notifications\Messages\MailMessage;
use Illuminate\Notifications\Notification;
use Illuminate\Support\Facades\Lang;

final class VerifyIdentityEmailChange extends Notification
{
    use Queueable;

    public function __construct(
        private readonly string $token,
        private readonly ?string $notificationLocale = null,
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
            ->subject((string) Lang::get('identity.mail.verify_subject', [], $locale))
            ->line((string) Lang::get('identity.mail.verify_intro', [], $locale))
            ->action(
                (string) Lang::get('identity.mail.verify_action', [], $locale),
                route('identity.email-change.confirm.create', ['token' => $this->token, 'locale' => $locale]),
            )
            ->line((string) Lang::get('identity.mail.verify_outro', [], $locale));
    }
}
