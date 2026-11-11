<?php

namespace App\Http\Middleware\GameAuth;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

/**
 * Early HTTP bounds for the native internal routes: 413 on a Content-Length or body above the route's
 * limit (native evidence by default; `:<bytes>` for another route), 400 on Transfer-Encoding, a
 * Content-Encoding other than identity or a non-JSON Content-Type, and 431 on oversized headers.
 */
final class EnforceNativeEvidenceHttpBounds
{
    public function handle(Request $request, Closure $next, ?string $maxRequestBytes = null): Response
    {
        $limit = $maxRequestBytes === null ? NativeEvidenceContract::MAX_REQUEST_BYTES : (int) $maxRequestBytes;
        $contentLength = $request->headers->get('Content-Length');
        if ($contentLength !== null
            && (preg_match('/^(0|[1-9][0-9]{0,3})$/', $contentLength) !== 1
                || (int) $contentLength > $limit)) {
            return response('', 413);
        }

        $contentType = $request->headers->get('Content-Type');
        if (! is_string($contentType) || strtolower(trim(explode(';', $contentType, 2)[0])) !== 'application/json') {
            return response('', 400);
        }

        $contentEncoding = $request->headers->get('Content-Encoding');
        if (is_string($contentEncoding) && strtolower(trim($contentEncoding)) !== 'identity') {
            return response('', 400);
        }
        if ($request->headers->has('Transfer-Encoding')) {
            return response('', 400);
        }

        $fieldCount = 0;
        $headerBytes = strlen($request->getMethod().' '.$request->getRequestUri().' HTTP/1.1\r\n') + 2;
        foreach ($request->headers->all() as $name => $values) {
            foreach ($values as $value) {
                if ($value === null) {
                    return response('', 400);
                }

                $fieldCount++;
                $lineBytes = strlen($name) + 2 + strlen($value) + 2;
                if ($lineBytes > 2048) {
                    return response('', 431);
                }
                $headerBytes += $lineBytes;
            }
        }
        if ($fieldCount > 32 || $headerBytes > 8192) {
            return response('', 431);
        }

        $body = $request->getContent();
        if (strlen($body) > $limit) {
            return response('', 413);
        }
        if ($contentLength !== null && (int) $contentLength !== strlen($body)) {
            return response('', 400);
        }

        $response = $next($request);
        if (! $response instanceof Response) {
            return response('', 500);
        }

        return $response;
    }
}
