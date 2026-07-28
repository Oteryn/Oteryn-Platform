<?php

namespace App\Marketplace\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class EnsureMarketplaceEnabled
{
    /** @param Closure(Request): Response $next */
    public function handle(Request $request, Closure $next): Response
    {
        abort_unless(config('marketplace.enabled') === true, 404);

        return $next($request);
    }
}
