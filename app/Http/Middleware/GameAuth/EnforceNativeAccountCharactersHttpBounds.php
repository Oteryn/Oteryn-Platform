<?php

namespace App\Http\Middleware\GameAuth;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire;
use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

final class EnforceNativeAccountCharactersHttpBounds
{
    public function handle(Request $request, Closure $next, string $kind = 'snapshot'): Response
    {
        $limit = $kind === 'watermark'
            ? NativeAccountCharactersWire::WATERMARK_MAX_BYTES
            : NativeAccountCharactersWire::SNAPSHOT_MAX_BYTES;

        $contentLength = $request->headers->get('Content-Length');
        if ($contentLength !== null
            && (preg_match('/^(0|[1-9][0-9]{0,4})$/D', $contentLength) !== 1
                || (int) $contentLength > $limit)) {
            return response('', 413);
        }

        $contentType = $request->headers->get('Content-Type');
        if (! is_string($contentType) || strtolower(trim(explode(';', $contentType, 2)[0])) !== 'application/json') {
            return response('', 400);
        }
        $encoding = $request->headers->get('Content-Encoding');
        if (is_string($encoding) && strtolower(trim($encoding)) !== 'identity') {
            return response('', 400);
        }
        if ($request->headers->has('Transfer-Encoding')) {
            return response('', 400);
        }

        $body = $request->getContent();
        if (strlen($body) > $limit) {
            return response('', 413);
        }
        if ($contentLength !== null && (int) $contentLength !== strlen($body)) {
            return response('', 400);
        }

        $response = $next($request);

        return $response instanceof Response ? $response : response('', 500);
    }
}
