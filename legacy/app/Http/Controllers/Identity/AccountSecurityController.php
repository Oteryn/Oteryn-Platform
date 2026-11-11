<?php

namespace App\Http\Controllers\Identity;

use App\Identity\Models\Identity;
use App\Identity\Models\IdentityEmailChangeRequest;
use App\Identity\Sessions\IdentityWebSessionRegistry;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Contracts\View\View;
use Illuminate\Http\Request;

final class AccountSecurityController
{
    public function show(Request $request, IdentityWebSessionRegistry $sessions): View
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);
        $identity->refresh();

        $pendingEmailChange = IdentityEmailChangeRequest::query()
            ->where('identity_id', $identity->id)
            ->whereNull('confirmed_at')
            ->whereNull('cancelled_at')
            ->whereNull('recovered_at')
            ->where('expires_at', '>', now())
            ->latest('requested_at')
            ->first();

        return view('identity.account.security', [
            'identity' => $identity,
            'sessions' => $sessions->activeFor($identity),
            'currentSessionId' => $request->session()->get(WebSessionState::REGISTRY_ID_KEY),
            'pendingEmailChange' => $pendingEmailChange,
            'hasRecoveryKey' => $identity->recoveryKey()->whereNull('used_at')->whereNull('revoked_at')->exists(),
            'bindingMutationPolicy' => config('identity_security.binding_mutation_policy'),
            'emailCodeMfaEnabled' => config('identity_security.email_code_mfa_enabled'),
        ]);
    }
}
