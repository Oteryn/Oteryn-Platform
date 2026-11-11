<?php

namespace App\Identity\Credentials;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;

final class IdentityCredentialUpdater
{
    public function __construct(
        private readonly RevokeIdentityWebSessions $webSessions,
        private readonly RevokeIdentityGameAuthorizations $gameAuthorizations,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function change(Identity $identity, string $newPassword): void
    {
        DB::transaction(function () use ($identity, $newPassword): void {
            $this->replacePassword($identity, $newPassword);
            $this->webSessions->execute($identity);
            $this->gameAuthorizations->execute($identity);
            $this->securityEvents->recordIdentityPasswordChanged($identity->id);
        });
    }

    public function reset(Identity $identity, string $newPassword): void
    {
        DB::transaction(function () use ($identity, $newPassword): void {
            $this->replacePassword($identity, $newPassword);
            $this->webSessions->execute($identity);
            $this->gameAuthorizations->execute($identity);
            $this->securityEvents->recordIdentityPasswordResetCompleted($identity->id);
        });
    }

    private function replacePassword(Identity $identity, string $newPassword): void
    {
        $identity->forceFill([
            'password' => Hash::make($newPassword),
        ])->save();
    }
}
