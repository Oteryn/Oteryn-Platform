<?php

namespace App\Http\Controllers\Identity\Mfa;

use App\Audit\SecurityEventRecorder;
use App\Http\Requests\Identity\Mfa\MfaChallengeRequest;
use App\Identity\Mfa\MfaCodeConsumer;
use App\Identity\Mfa\MfaCodeRejected;
use App\Identity\Mfa\MfaStateRejected;
use App\Identity\Mfa\PendingMfaLogin;
use App\Identity\Sessions\IdentityWebSessionManager;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;

final class MfaChallengeController
{
    public function create(Request $request, PendingMfaLogin $pendingMfa): Response|RedirectResponse
    {
        if ($pendingMfa->state($request) === null) {
            return redirect()
                ->route('identity.login.create')
                ->with('status', 'Your sign-in challenge has expired. Please sign in again.');
        }

        return response()
            ->view('identity.mfa.challenge')
            ->header('Cache-Control', 'no-store, private')
            ->header('Pragma', 'no-cache');
    }

    public function store(
        MfaChallengeRequest $request,
        PendingMfaLogin $pendingMfa,
        MfaCodeConsumer $codes,
        IdentityWebSessionManager $sessions,
        SecurityEventRecorder $securityEvents,
    ): RedirectResponse {
        $state = $pendingMfa->state($request);

        if ($state === null) {
            return redirect()
                ->route('identity.login.create')
                ->with('status', 'Your sign-in challenge has expired. Please sign in again.');
        }

        try {
            $identity = $codes->consumeForPendingLogin(
                $state['identity_id'],
                $state['generation'],
                $state['confirmed_at'],
                $request->code(),
            );
        } catch (MfaCodeRejected) {
            return back()->withErrors([
                'code' => 'The verification code is invalid or has already been used.',
            ]);
        } catch (MfaStateRejected) {
            $pendingMfa->clear($request);

            return redirect()
                ->route('identity.login.create')
                ->with('status', 'Your sign-in challenge is no longer valid. Please sign in again.');
        }

        $pendingMfa->clear($request);
        $sessions->login($identity);
        $sessions->establish($request, $identity);
        $securityEvents->recordIdentityLoginSucceeded($identity->id);

        return redirect()->intended('/');
    }
}
