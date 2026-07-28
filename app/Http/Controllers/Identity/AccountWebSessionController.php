<?php

namespace App\Http\Controllers\Identity;

use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use App\Identity\Sessions\IdentityWebSessionManager;
use App\Identity\Sessions\IdentityWebSessionRegistry;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;

final class AccountWebSessionController
{
    public function destroy(
        Request $request,
        string $session,
        IdentityWebSessionRegistry $registry,
        IdentityWebSessionManager $sessions,
        SecurityEventRecorder $securityEvents,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $currentSessionId = $registry->currentId($request);
        $revoked = $registry->revokeOwned($identity, $session);
        if (! $revoked) {
            return redirect()
                ->route('identity.account-security.show')
                ->withErrors(['session' => __('identity.errors.selected_session_inactive')]);
        }

        $securityEvents->recordIdentityWebSessionRevoked($identity->id);

        if ($currentSessionId === $session) {
            $sessions->invalidate($request);

            return redirect()
                ->route('identity.login.create')
                ->with('status', __('identity.status.session_revoked_sign_in'));
        }

        return redirect()
            ->route('identity.account-security.show')
            ->with('status', __('identity.status.session_revoked'));
    }

    public function destroyOthers(
        Request $request,
        IdentityWebSessionRegistry $registry,
        SecurityEventRecorder $securityEvents,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $currentSessionId = $registry->currentId($request);
        abort_unless(is_string($currentSessionId), 403);

        $registry->revokeOthers($identity, $currentSessionId);
        $securityEvents->recordIdentityOtherWebSessionsRevoked($identity->id);

        return redirect()
            ->route('identity.account-security.show')
            ->with('status', __('identity.status.other_sessions_revoked'));
    }
}
