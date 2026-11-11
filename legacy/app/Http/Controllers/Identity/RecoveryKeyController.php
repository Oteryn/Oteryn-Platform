<?php

namespace App\Http\Controllers\Identity;

use App\Http\Requests\Identity\ManageRecoveryKeyRequest;
use App\Http\Requests\Identity\RecoverWithIdentityKeyRequest;
use App\Identity\Models\Identity;
use App\Identity\Recovery\IdentityRecoveryKeyService;
use App\Identity\Recovery\RecoveryKeyRejected;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Response;

final class RecoveryKeyController
{
    public function generate(
        ManageRecoveryKeyRequest $request,
        IdentityRecoveryKeyService $recoveryKeys,
    ): Response|RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $rawKey = $recoveryKeys->generate($identity);
        } catch (RecoveryKeyRejected $exception) {
            return redirect()
                ->route('identity.account-security.show')
                ->withErrors(['recovery_key' => $exception->getMessage()]);
        }

        return response()
            ->view('identity.recovery-key.generated', ['recoveryKey' => $rawKey])
            ->header('Cache-Control', 'no-store, private')
            ->header('Pragma', 'no-cache');
    }

    public function revoke(
        ManageRecoveryKeyRequest $request,
        IdentityRecoveryKeyService $recoveryKeys,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $recoveryKeys->revoke($identity);
        } catch (RecoveryKeyRejected $exception) {
            return redirect()
                ->route('identity.account-security.show')
                ->withErrors(['recovery_key' => $exception->getMessage()]);
        }

        return redirect()
            ->route('identity.account-security.show')
            ->with('status', __('identity.status.recovery_key_revoked'));
    }

    public function recoverCreate(): View
    {
        return view('identity.recovery-key.recover');
    }

    public function recover(
        RecoverWithIdentityKeyRequest $request,
        IdentityRecoveryKeyService $recoveryKeys,
    ): RedirectResponse {
        try {
            $recoveryKeys->recover(
                $request->canonicalEmail(),
                $request->recoveryKey(),
                $request->newPassword(),
            );
        } catch (RecoveryKeyRejected) {
            return back()
                ->withInput($request->safe()->only('email'))
                ->withErrors(['recovery_key' => __('identity.errors.recovery_credentials_invalid')]);
        }

        return redirect()
            ->route('identity.login.create')
            ->with('status', __('identity.status.account_recovered'));
    }
}
