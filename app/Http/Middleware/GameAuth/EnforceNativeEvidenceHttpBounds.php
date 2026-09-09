<?php

namespace App\Http\Middleware\GameAuth;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class EnforceNativeEvidenceHttpBounds
{
    public function handle(Request $request, Closure $next): Response
    {
        $contentLength = $request->headers->get('Content-Length');
        if ($contentLength !== null
            && (! is_string($contentLength)
                || preg_match('/^(0|[1-9][0-9]{0,3})$/', $contentLength) !== 1
                || (int) $contentLength > NativeEvidenceContract::MAX_REQUEST_BYTES)) {
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
        if (strlen($body) > NativeEvidenceContract::MAX_REQUEST_BYTES) {
            return response('', 413);
        }
        if ($contentLength !== null && (int) $contentLength !== strlen($body)) {
            return response('', 400);
        }

        return $next($request);
    }
}
