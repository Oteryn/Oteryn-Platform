<?php

namespace App\Notifications\Identity;

use Illuminate\Bus\Queueable;
use Illuminate\Notifications\Messages\MailMessage;
use Illuminate\Notifications\Notification;

final class VerifyIdentityEmailChange extends Notification
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
            ->subject('Confirm your new Oteryn email address')
            ->line('A primary email change was requested for an Oteryn Platform account.')
            ->action('Confirm new email address', route('identity.email-change.confirm.create', ['token' => $this->token]))
            ->line('This verification link expires automatically. Ignore this message when you did not request the change.');
    }
}
