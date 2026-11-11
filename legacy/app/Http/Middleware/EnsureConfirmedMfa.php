<?php

namespace App\Http\Middleware;

use App\Identity\Models\Identity;
use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class EnsureConfirmedMfa
{
    /**
     * @param  Closure(Request): Response  $next
     */
    public function handle(Request $request, Closure $next): Response
    {
        $identity = $request->user();

        if (! $identity instanceof Identity || ! $identity->hasConfirmedMfa()) {
            abort(403);
        }

        return $next($request);
    }
}
