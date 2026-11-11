<?php

namespace App\Identity\Credentials;

use Illuminate\Support\Facades\Password;
use RuntimeException;

final class PasswordResetLinkSender
{
    public function send(string $email): void
    {
        $mailer = config('mail.default');

        if (! is_string($mailer) || $mailer === '') {
            throw new RuntimeException('The default mailer is not configured.');
        }

        $transport = config("mail.mailers.{$mailer}.transport");

        if ($transport === 'log') {
            throw new RuntimeException('Password reset links must not use the log mail transport.');
        }

        Password::broker('identities')->sendResetLink([
            'email' => $email,
        ]);
    }
}
