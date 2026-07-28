<?php

namespace App\Http\Controllers\Identity;

use App\Http\Requests\Identity\RequestEmailChangeRequest;
use App\Identity\Email\ConfirmIdentityEmailChange;
use App\Identity\Email\EmailChangeRejected;
use App\Identity\Email\RecoverIdentityEmailChange;
use App\Identity\Email\RequestIdentityEmailChange;
use App\Identity\Models\Identity;
use App\Identity\Sessions\IdentityWebSessionManager;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;

final class EmailChangeController
{
    public function store(
        RequestEmailChangeRequest $request,
        RequestIdentityEmailChange $emailChanges,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $emailChanges->execute($identity, $request->canonicalEmail());
        } catch (EmailChangeRejected $exception) {
            return redirect()
                ->route('identity.account-security.show')
                ->withErrors(['email' => $exception->getMessage()]);
        }

        return redirect()
            ->route('identity.account-security.show')
            ->with('status', 'Check the new email address to confirm the change. The old address received a recovery link.');
    }

    public function confirmCreate(string $token): View
    {
        return view('identity.email-change.confirm', ['token' => $token]);
    }

    public function confirm(
        Request $request,
        string $token,
        ConfirmIdentityEmailChange $emailChanges,
        IdentityWebSessionManager $sessions,
    ): RedirectResponse {
        try {
            $emailChanges->execute($token);
        } catch (EmailChangeRejected $exception) {
            return redirect()
                ->route('identity.email-change.confirm.create', ['token' => $token])
                ->withErrors(['email' => $exception->getMessage()]);
        }

        if ($request->user() instanceof Identity) {
            $sessions->invalidate($request);
        }

        return redirect()
            ->route('identity.login.create')
            ->with('status', 'Your primary email address has been changed. Sign in with the new address.');
    }

    public function recoverCreate(string $token): View
    {
        return view('identity.email-change.recover', ['token' => $token]);
    }

    public function recover(
        string $token,
        RecoverIdentityEmailChange $emailChanges,
    ): RedirectResponse {
        try {
            $result = $emailChanges->execute($token);
        } catch (EmailChangeRejected $exception) {
            return redirect()
                ->route('identity.email-change.recover.create', ['token' => $token])
                ->withErrors(['email' => $exception->getMessage()]);
        }

        return redirect()
            ->route('identity.login.create')
            ->with('status', $result === 'recovered'
                ? 'The previous email address has been restored. Sign in again.'
                : 'The pending email change has been cancelled.');
    }
}
