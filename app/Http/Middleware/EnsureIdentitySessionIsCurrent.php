<?php

namespace App\Http\Middleware;

use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use App\Identity\Sessions\IdentityWebSessionManager;
use App\Identity\Sessions\IdentityWebSessionRegistry;
use App\Identity\Sessions\WebSessionState;
use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class EnsureIdentitySessionIsCurrent
{
    public function __construct(
        private readonly IdentityWebSessionManager $sessions,
        private readonly IdentityWebSessionRegistry $registry,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    /**
     * @param  Closure(Request): Response  $next
     */
    public function handle(Request $request, Closure $next): Response
    {
        $authenticatedIdentity = $this->sessions->user();

        if ($authenticatedIdentity === null) {
            return $next($request);
        }

        if (! $authenticatedIdentity instanceof Identity) {
            $this->sessions->invalidate($request);

            return $next($request);
        }

        $identityId = $authenticatedIdentity->id;
        $currentIdentity = Identity::query()->find($identityId);
        $sessionGeneration = $request->session()->get(WebSessionState::GENERATION_KEY);
        $registeredSession = $currentIdentity instanceof Identity
            ? $this->registry->current($request, $currentIdentity)
            : null;
        $sessionIsCurrent = $currentIdentity instanceof Identity
            && is_int($sessionGeneration)
            && $sessionGeneration === $currentIdentity->web_session_generation
            && $currentIdentity->disabled_at === null
            && $currentIdentity->terminated_at === null
            && $registeredSession !== null
            && $registeredSession->generation === $currentIdentity->web_session_generation
            && $registeredSession->isActive();

        if (! $sessionIsCurrent) {
            $this->sessions->invalidate($request);
            $this->securityEvents->recordIdentityWebSessionRejected($identityId);

            return $next($request);
        }

        $this->registry->touch($registeredSession);

        return $next($request);
    }
}
