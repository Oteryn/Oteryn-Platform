<?php

namespace App\Http\Controllers\Identity;

use App\Http\Requests\Identity\ChangePasswordRequest;
use App\Identity\Credentials\IdentityCredentialUpdater;
use App\Identity\Models\Identity;
use App\Identity\Sessions\IdentityWebSessionManager;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;

final class PasswordChangeController
{
    public function create(): View
    {
        return view('identity.change-password');
    }

    public function update(
        ChangePasswordRequest $request,
        IdentityCredentialUpdater $credentials,
        IdentityWebSessionManager $sessions,
    ): RedirectResponse {
        $identity = $request->user();

        abort_unless($identity instanceof Identity, 403);

        $credentials->change($identity, $request->newPassword());
        $sessions->invalidate($request);

        return redirect()
            ->route('identity.login.create')
            ->with('status', 'Your password has been changed. Sign in again.');
    }
}
