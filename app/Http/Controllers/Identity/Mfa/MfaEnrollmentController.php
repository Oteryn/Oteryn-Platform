<?php

namespace App\Http\Controllers\Identity\Mfa;

use App\Http\Requests\Identity\Mfa\ConfirmMfaEnrollmentRequest;
use App\Http\Requests\Identity\Mfa\DisableMfaRequest;
use App\Identity\Mfa\ConfirmIdentityMfaEnrollment;
use App\Identity\Mfa\DisableIdentityMfa;
use App\Identity\Mfa\MfaCodeRejected;
use App\Identity\Mfa\MfaProvisioningUri;
use App\Identity\Mfa\MfaStateRejected;
use App\Identity\Mfa\StartIdentityMfaEnrollment;
use App\Identity\Models\Identity;
use App\Identity\Sessions\IdentityWebSessionManager;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;

final class MfaEnrollmentController
{
    public function show(Request $request, MfaProvisioningUri $provisioningUri): Response
    {
        $identity = $this->identity($request);
        $identity->refresh();
        $uri = null;

        if (! $identity->hasConfirmedMfa() && $identity->two_factor_secret !== null) {
            $uri = $provisioningUri->forIdentity($identity);
        }

        return response()
            ->view('identity.mfa.settings', [
                'identity' => $identity,
                'provisioningUri' => $uri,
            ])
            ->header('Cache-Control', 'no-store, private')
            ->header('Pragma', 'no-cache');
    }

    public function store(
        Request $request,
        StartIdentityMfaEnrollment $enrollment,
    ): RedirectResponse {
        try {
            $enrollment->execute($this->identity($request));
        } catch (MfaStateRejected) {
            return back()->withErrors([
                'mfa' => 'MFA enrollment cannot be started for this identity.',
            ]);
        }

        return redirect()
            ->route('identity.mfa.settings')
            ->with('status', 'Enter this secret in your authenticator app, then confirm with your current password and a fresh code.');
    }

    public function confirm(
        ConfirmMfaEnrollmentRequest $request,
        ConfirmIdentityMfaEnrollment $enrollment,
        IdentityWebSessionManager $sessions,
    ): Response|RedirectResponse {
        try {
            $confirmation = $enrollment->execute(
                $this->identity($request),
                $request->currentPassword(),
                $request->code(),
            );
        } catch (MfaCodeRejected|MfaStateRejected) {
            return back()->withErrors([
                'code' => 'The current password or verification code is invalid.',
            ]);
        }

        $sessions->establish($request, $confirmation->identity);

        return response()
            ->view('identity.mfa.recovery-codes', [
                'recoveryCodes' => $confirmation->recoveryCodes,
            ])
            ->header('Cache-Control', 'no-store, private')
            ->header('Pragma', 'no-cache');
    }

    public function destroy(
        DisableMfaRequest $request,
        DisableIdentityMfa $disableMfa,
        IdentityWebSessionManager $sessions,
    ): RedirectResponse {
        try {
            $disableMfa->execute(
                $this->identity($request),
                $request->currentPassword(),
                $request->code(),
            );
        } catch (MfaCodeRejected|MfaStateRejected) {
            return back()->withErrors([
                'code' => 'The current password or verification code is invalid.',
            ]);
        }

        $sessions->invalidate($request);

        return redirect()
            ->route('home')
            ->with('status', 'Multi-factor authentication has been disabled. Sign in again to continue.');
    }

    private function identity(Request $request): Identity
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return $identity;
    }
}
