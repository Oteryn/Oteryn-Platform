<?php

namespace App\Http\Controllers\Identity;

use App\Http\Requests\Identity\ManageRecoveryKeyRequest;
use App\Http\Requests\Identity\RequestAccountTerminationRequest;
use App\Identity\Models\Identity;
use App\Identity\Sessions\IdentityWebSessionManager;
use App\Identity\Termination\AccountTerminationRejected;
use App\Identity\Termination\CancelIdentityTermination;
use App\Identity\Termination\RequestIdentityTermination;
use Illuminate\Http\RedirectResponse;

final class AccountTerminationController
{
    public function store(
        RequestAccountTerminationRequest $request,
        RequestIdentityTermination $termination,
        IdentityWebSessionManager $sessions,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $scheduled = $termination->execute($identity);
        } catch (AccountTerminationRejected $exception) {
            return redirect()
                ->route('identity.account-security.show')
                ->withErrors(['termination' => $exception->getMessage()]);
        }

        $sessions->invalidate($request);

        return redirect()
            ->route('identity.login.create')
            ->with('status', __('identity.status.termination_scheduled', [
                'date' => $scheduled->termination_scheduled_for?->utc()->format('Y-m-d H:i'),
            ]));
    }

    public function destroy(
        ManageRecoveryKeyRequest $request,
        CancelIdentityTermination $termination,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $termination->execute($identity);
        } catch (AccountTerminationRejected $exception) {
            return redirect()
                ->route('identity.account-security.show')
                ->withErrors(['termination' => $exception->getMessage()]);
        }

        return redirect()
            ->route('identity.account-security.show')
            ->with('status', __('identity.status.termination_cancelled'));
    }
}
