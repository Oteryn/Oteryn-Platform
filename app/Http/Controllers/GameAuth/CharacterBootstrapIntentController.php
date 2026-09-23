<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentReader;
use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentRequestDecoder;
use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentUnavailable;
use Illuminate\Database\QueryException;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;

final class CharacterBootstrapIntentController
{
    public function __invoke(
        Request $request,
        CharacterBootstrapIntentRequestDecoder $decoder,
        CharacterBootstrapIntentReader $reader,
    ): JsonResponse|Response {
        try {
            $decoded = $decoder->decode($request->getContent());
        } catch (InvalidArgumentException) {
            return response('', 400);
        }

        try {
            $payload = $reader->current($decoded['operation_id']);
        } catch (CharacterBootstrapIntentUnavailable|QueryException) {
            return response('', 503);
        }
        if ($payload === null) {
            return response('', 404);
        }

        return response()->json($payload, 200, [], JSON_UNESCAPED_SLASHES);
    }
}
