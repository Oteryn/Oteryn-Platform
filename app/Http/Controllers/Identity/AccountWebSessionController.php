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
                ->withErrors(['session' => 'The selected session is no longer active.']);
        }

        $securityEvents->recordIdentityWebSessionRevoked($identity->id);

        if ($currentSessionId === $session) {
            $sessions->invalidate($request);

            return redirect()
                ->route('identity.login.create')
                ->with('status', 'This session has been revoked. Sign in again to continue.');
        }

        return redirect()
            ->route('identity.account-security.show')
            ->with('status', 'The selected session has been revoked.');
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
            ->with('status', 'All other active sessions have been revoked.');
    }
}
