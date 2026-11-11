<?php

namespace App\Http\Controllers\GameAuth;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReadModel;
use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use App\GameAuth\Worlds\NativeRouteRecord;
use App\GameAuth\Worlds\NativeRouteRecords;
use App\GameAuth\Worlds\NativeTopologyReceipt;
use Illuminate\Http\JsonResponse;
use Symfony\Component\HttpFoundation\Response;

/** Public availability only; account characters require authenticated owner reads. */
final class ClientDirectoryController
{
    public function __invoke(NativeRouteRecords $routes, NativeRuntimeStatusReadModel $runtime): JsonResponse|Response
    {
        // Native Registry routes have no production activation authority yet.
        if (! app()->environment(['testing', 'preproduction'])) {
            return response('', 503);
        }

        $now = now();
        $worlds = GameWorld::query()->whereNotNull('world_id')
            ->where('login_enabled', true)->where('status', GameWorldStatus::Online->value)
            ->orderBy('world_id')->limit(257)->get();
        if ($worlds->count() > 256) {
            return response('', 503);
        }

        $public = [];
        foreach ($worlds as $world) {
            if (! is_string($world->world_id) || ! NativeTopologyReceipt::isCanonicalId($world->world_id)) {
                continue;
            }
            if (! self::validName($world->name)) {
                return response('', 503);
            }
            $channels = [];
            $storedChannels = $world->channels()->orderBy('channel_id')->limit(129)->get();
            if ($storedChannels->count() > 128) {
                return response('', 503);
            }
            foreach ($storedChannels as $channel) {
                if (! NativeTopologyReceipt::isCanonicalId($channel->channel_id)
                    || $channel->channel_id === $world->world_id || ! self::validName($channel->channel_key)) {
                    return response('', 503);
                }
                $route = $routes->find($world->world_id, $channel->channel_id);
                $report = $runtime->routable($world->world_id, $channel->channel_id, $now->getTimestamp());
                if ($route === null || $report === null
                    || ! hash_equals($route->routeRevision, $report->routeRevision)
                    || $report->protocolMajor !== NativeRouteRecord::PROTOCOL_MAJOR
                    || $report->transportProfile !== NativeRouteRecord::TRANSPORT_PROFILE) {
                    continue;
                }
                $channels[] = ['id' => $channel->channel_id, 'name' => $channel->channel_key];
            }
            if ($channels !== []) {
                $public[] = ['id' => $world->world_id, 'name' => $world->name,
                    'channels' => $channels, 'characters' => []];
            }
        }

        $response = response()->json(['epoch' => $now->getTimestampMs(), 'worlds' => $public]);
        if (strlen((string) $response->getContent()) > 1_048_576) {
            return response('', 503);
        }

        return $response->header('Cache-Control', 'no-store, private')->header('Pragma', 'no-cache');
    }

    private static function validName(string $name): bool
    {
        return $name !== '' && strlen($name) <= 96 && preg_match('/\p{Cc}/u', $name) === 0;
    }
}
