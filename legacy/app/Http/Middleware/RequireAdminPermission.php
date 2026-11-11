<?php

namespace App\Http\Middleware;

use App\Admin\AdminAuthorization;
use App\Identity\Models\Identity;
use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class RequireAdminPermission
{
    public function __construct(private readonly AdminAuthorization $authorization) {}

    /**
     * @param  Closure(Request): Response  $next
     */
    public function handle(Request $request, Closure $next, string $permission): Response
    {
        $identity = $request->user();

        if (! $identity instanceof Identity || ! $this->authorization->allows($identity, $permission)) {
            abort(403);
        }

        return $next($request);
    }
}
