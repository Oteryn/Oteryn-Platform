<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharacterSummary;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersAccountView;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel;
use App\GameAuth\OAuth\OAuthBootstrapDenied;
use App\GameAuth\OAuth\VerifyNativeOAuthAccess;
use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalAccountId;
use Illuminate\Database\QueryException;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;
use JsonException;
use Laravel\Passport\AccessToken;
use Symfony\Component\HttpFoundation\Response;
use UnexpectedValueException;

final class NativeAccountCharactersReadController
{
    private const MAX_RESPONSE_BYTES = 16384;

    public function __invoke(
        Request $request,
        VerifyNativeOAuthAccess $access,
        NativeAccountCharactersReadModel $characters,
    ): Response {
        $identity = $request->user('api');
        if (! $identity instanceof Identity) {
            return response()->json(['error' => 'unauthenticated'], 401);
        }

        $accessToken = $identity->currentAccessToken();
        if (! $accessToken instanceof AccessToken) {
            return response()->json(['error' => 'unauthenticated'], 401);
        }
        $attributes = $accessToken->toArray();
        $accessTokenId = $attributes['oauth_access_token_id'] ?? null;
        if (! is_string($accessTokenId) || $accessTokenId === '') {
            return response()->json(['error' => 'unauthenticated'], 401);
        }

        try {
            $view = DB::transaction(function () use ($access, $accessTokenId, $characters, $identity): NativeAccountCharactersAccountView {
                $verified = $access->locked($identity, $accessTokenId);
                $accountId = $verified->identity->account_id;
                if (! CanonicalAccountId::isValid($accountId)) {
                    throw new OAuthBootstrapDenied;
                }

                return $characters->viewForAccount($accountId, now()->getTimestamp());
            });
        } catch (OAuthBootstrapDenied) {
            return response()->json(['error' => 'unauthenticated'], 401);
        } catch (QueryException|UnexpectedValueException) {
            return response('', 503);
        }

        if (! in_array($view->state, [NativeAccountCharactersAccountView::READY, NativeAccountCharactersAccountView::MISSING], true)) {
            return response('', 503);
        }

        $payload = [
            'protocol_version' => 2,
            'characters' => array_map(static fn (NativeAccountCharacterSummary $character): array => [
                'character_id' => $character->characterId,
                'world_id' => $character->worldId,
                'name' => $character->name,
                'availability' => $character->availability,
            ], $view->characters),
        ];

        try {
            $encoded = json_encode($payload, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        } catch (JsonException) {
            return response('', 503);
        }
        if (strlen($encoded) > self::MAX_RESPONSE_BYTES) {
            return response('', 503);
        }

        return new JsonResponse($payload, 200, [], JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }
}
