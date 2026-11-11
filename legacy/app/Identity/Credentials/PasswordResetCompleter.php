<?php

namespace App\Identity\Credentials;

use App\Identity\Models\Identity;
use Illuminate\Contracts\Auth\CanResetPassword as CanResetPasswordContract;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Password;
use RuntimeException;

final class PasswordResetCompleter
{
    public function __construct(
        private readonly IdentityCredentialUpdater $credentials,
    ) {}

    /**
     * @param  array{email: string, token: string, password: string}  $input
     */
    public function complete(array $input): string
    {
        return DB::transaction(function () use ($input): string {
            $status = Password::broker('identities')->reset(
                $input,
                function (CanResetPasswordContract $resettable, mixed $newPassword): void {
                    if (! $resettable instanceof Identity || ! is_string($newPassword)) {
                        throw new RuntimeException('The password reset identity is invalid.');
                    }

                    $this->credentials->reset($resettable, $newPassword);
                },
            );

            if (! is_string($status)) {
                throw new RuntimeException('The password broker returned an invalid reset status.');
            }

            return $status;
        });
    }
}
