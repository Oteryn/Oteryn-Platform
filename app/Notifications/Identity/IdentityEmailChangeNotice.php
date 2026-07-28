<?php

namespace App\Notifications\Identity;

use Illuminate\Bus\Queueable;
use Illuminate\Notifications\Messages\MailMessage;
use Illuminate\Notifications\Notification;

final class IdentityEmailChangeNotice extends Notification
{
    use Queueable;

    public function __construct(
        private readonly string $token,
    ) {}

    /** @return list<string> */
    public function via(object $notifiable): array
    {
        return ['mail'];
    }

    public function toMail(object $notifiable): MailMessage
    {
        return (new MailMessage)
            ->subject('Oteryn primary email change requested')
            ->line('A request was made to change the primary email address for your Oteryn Platform account.')
            ->action('Cancel or recover this change', route('identity.email-change.recover.create', ['token' => $this->token]))
            ->line('Use the recovery link immediately when you did not request this change. The link is single-use and expires automatically.');
    }
}
