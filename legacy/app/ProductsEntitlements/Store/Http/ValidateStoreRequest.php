<?php

namespace App\ProductsEntitlements\Store\Http;

use App\ProductsEntitlements\Store\StoreWire;
use Closure;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;

/** Transport and canonical head only; route business handlers are implemented in subsequent lanes. */
final class ValidateStoreRequest
{
    /** @param Closure(Request): Response $next */
    public function handle(Request $request, Closure $next, string $operation, string $bound): Response
    {
        $max = (int) $bound;
        $length = $request->headers->get('Content-Length');
        if ($length !== null && (preg_match('/^(0|[1-9][0-9]{0,9})$/D', $length) !== 1 || (int) $length > $max)) {
            return response('', 413);
        }
        $encoding = $request->headers->get('Content-Encoding');
        $type = $request->headers->get('Content-Type');
        if ($request->headers->has('Transfer-Encoding') || ($encoding !== null && strtolower(trim($encoding)) !== 'identity')
            || $type === null || strtolower(trim(explode(';', $type, 2)[0])) !== 'application/json') {
            return response('', 400);
        }
        try {
            $source = StoreWire::sourceAuthority($request->getContent(), $max, $operation);
        } catch (InvalidArgumentException) {
            return response('', 400);
        }
        if ($source !== config('products-entitlements.store.source_authority')) {
            return response('', 401);
        }

        return $next($request);
    }
}
