<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\Context\GameLoginCharacter;
use App\GameAuth\Context\GameLoginContextProvider;
use App\GameAuth\Context\GameLoginContextUnavailable;
use App\GameAuth\Worlds\GameWorldProtocolCandidateRoute;
use Illuminate\Database\QueryException;
use Illuminate\Http\JsonResponse;

final class GameLoginContextController
{
    public function __invoke(string $canaryAccountId, GameLoginContextProvider $contexts): JsonResponse
    {
        if (! ctype_digit($canaryAccountId) || (int) $canaryAccountId < 1) {
            return response()->json(['error' => 'invalid_account'], 422);
        }

        try {
            $context = $contexts->forAccount((int) $canaryAccountId);
        } catch (GameLoginContextUnavailable $exception) {
            return response()->json(['error' => $exception->reason], $exception->httpStatus);
        } catch (QueryException) {
            return response()->json(['error' => 'login_context_unavailable'], 503);
        }

        $policy = $context->world->gameplayPolicy;

        return response()->json([
            'protocol_version' => 1,
            'worlds' => [[
                'id' => $context->world->id,
                'slug' => $context->world->slug,
                'name' => $context->world->name,
                'region' => $context->world->region,
                'host' => $context->world->host,
                'port' => $context->world->port,
            ]],
            'characters' => array_map(
                static fn (GameLoginCharacter $character): array => [
                    'id' => $character->id,
                    'name' => $character->name,
                    'level' => $character->level,
                    'vocation' => $character->vocation,
                    'world_id' => $character->worldId,
                ],
                $context->characters,
            ),
            'gameplay_policy' => [
                'revision' => $policy->revision,
                'channel_id' => $policy->channelId,
                'candidates' => array_map(
                    static fn (GameWorldProtocolCandidateRoute $candidate): array => [
                        'family' => $candidate->family,
                        'profile' => $candidate->profile,
                        'transport' => $candidate->transport,
                        'schema_revision' => $candidate->schemaRevision,
                        'schema_sha256' => $candidate->schemaSha256,
                        'required_capabilities' => $candidate->requiredCapabilities,
                        'optional_capabilities' => $candidate->optionalCapabilities,
                        'endpoint_id' => $candidate->endpointId,
                        'host' => $candidate->host,
                        'port' => $candidate->port,
                        'tls_server_name' => $candidate->tlsServerName,
                    ],
                    $policy->candidates,
                ),
            ],
        ]);
    }
}
