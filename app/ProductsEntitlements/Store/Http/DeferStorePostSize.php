<?php

namespace App\ProductsEntitlements\Store\Http;

use Closure;
use Illuminate\Http\Middleware\ValidatePostSize;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

/** Store size validation must follow flags, peer authentication and rate limiting (wire §4.2). */
final class DeferStorePostSize
{
    public function __construct(private readonly ValidatePostSize $fallback) {}

    /** @param Closure(Request): Response $next */
    public function handle(Request $request, Closure $next): Response
    {
        if ($request->isMethod('POST') && in_array($request->path(), [
            'internal/v1/products-entitlements/store/balance/read',
            'internal/v1/products-entitlements/store/history/read',
            'internal/v1/products-entitlements/store/purchases',
            'internal/v1/products-entitlements/store/operations/pending',
            'internal/v1/products-entitlements/store/operations/read',
            'internal/v1/products-entitlements/store/operations/receipt',
            'internal/v1/products-entitlements/store/operations/reconcile',
        ], true)) {
            return $next($request);
        }

        $response = $this->fallback->handle($request, $next);

        return $response instanceof Response ? $response : response('', 500);
    }
}
