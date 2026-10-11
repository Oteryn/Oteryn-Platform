<?php

namespace App\GameProjections\Http;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

/** Request framing and the per-route byte bound (§8); a body over the bound is `400` (§4.0). */
final class EnforcePublicProjectionHttpBounds
{
    public function handle(Request $request, Closure $next, string $maxBytes): Response
    {
        $limit = (int) $maxBytes;
        $contentLength = $request->headers->get('Content-Length');
        if ($contentLength !== null
            && (preg_match('/^(0|[1-9][0-9]{0,6})$/D', $contentLength) !== 1 || (int) $contentLength > $limit)) {
            return response('', 400);
        }

        $contentType = $request->headers->get('Content-Type');
        $encoding = $request->headers->get('Content-Encoding');
        if (! is_string($contentType)
            || strtolower(trim(explode(';', $contentType, 2)[0])) !== 'application/json'
            || (is_string($encoding) && strtolower(trim($encoding)) !== 'identity')
            || $request->headers->has('Transfer-Encoding')) {
            return response('', 400);
        }

        $body = $request->getContent();
        if (strlen($body) > $limit || ($contentLength !== null && (int) $contentLength !== strlen($body))) {
            return response('', 400);
        }

        $response = $next($request);

        return $response instanceof Response ? $response : response('', 500);
    }
}
