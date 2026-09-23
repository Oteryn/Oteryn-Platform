<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentRequestDecoder;
use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentService;
use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentUnavailable;
use Illuminate\Database\QueryException;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;

final class CharacterBootstrapIntentController
{
    public function __invoke(Request $request, CharacterBootstrapIntentRequestDecoder $decoder, CharacterBootstrapIntentService $service): JsonResponse|Response
    {
        $type = $request->headers->get('Content-Type');
        if (! is_string($type) || strtolower(trim(explode(';', $type, 2)[0])) !== 'application/json'
            || $request->headers->has('Transfer-Encoding') || strlen($request->getContent()) > 512) {
            return response('', 400);
        }
        try {
            $decoded = $decoder->decode($request->getContent());
            $payload = $service->current($decoded['operation_id']);
        } catch (InvalidArgumentException) {
            return response('', 400);
        } catch (CharacterBootstrapIntentUnavailable|QueryException) {
            return response('', 404);
        }

        return response()->json($payload, 200, ['Cache-Control' => 'no-store, private', 'Pragma' => 'no-cache'], JSON_UNESCAPED_SLASHES);
    }
}
