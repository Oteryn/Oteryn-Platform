<?php

namespace App\LiveOps\WorldStatus;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimePublicEvidence;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReadModel;
use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use App\GameAuth\Worlds\NativeTopologyReceipt;

/**
 * Public-safe LiveOps projection over Platform topology/policy and the accepted native runtime read model.
 * This query never reads raw runtime tables and never exposes private runtime owner/fencing/route data.
 */
final readonly class PublicWorldStatusQuery
{
    public function __construct(private NativeRuntimeStatusReadModel $runtime) {}

    /** @return list<PublicWorldStatus> */
    public function get(int $now): array
    {
        $projected = [];
        $worlds = GameWorld::query()->whereNotNull('world_id')->orderBy('id')->get();

        foreach ($worlds as $world) {
            if (! NativeTopologyReceipt::isCanonicalId($world->world_id)) {
                continue;
            }

            $evidence = [];
            foreach ($world->channels()->orderBy('id')->get() as $channel) {
                if (! NativeTopologyReceipt::isCanonicalId($channel->channel_id)
                    || $channel->channel_id === $world->world_id) {
                    continue;
                }
                $evidence[] = $this->runtime->publicEvidence($world->world_id, $channel->channel_id, $now);
            }

            [$runtimeState, $observedAt] = $this->aggregate($evidence);
            $projected[] = new PublicWorldStatus(
                worldId: $world->world_id,
                slug: $world->slug,
                name: $world->name,
                policyState: $this->policyState($world),
                runtimeState: $runtimeState,
                observedAt: $observedAt,
            );
        }

        return $projected;
    }

    private function policyState(GameWorld $world): string
    {
        return match ($world->status) {
            GameWorldStatus::Maintenance => PublicWorldStatus::POLICY_MAINTENANCE,
            GameWorldStatus::Offline => PublicWorldStatus::POLICY_OFFLINE,
            GameWorldStatus::Unknown => PublicWorldStatus::POLICY_UNKNOWN,
            GameWorldStatus::Online => $world->login_enabled
                ? PublicWorldStatus::POLICY_ONLINE
                : PublicWorldStatus::POLICY_DISABLED,
        };
    }

    /**
     * @param  list<NativeRuntimePublicEvidence>  $evidence
     * @return array{0:string,1:int|null}
     */
    private function aggregate(array $evidence): array
    {
        if ($evidence === []) {
            return [PublicWorldStatus::RUNTIME_UNAVAILABLE, null];
        }

        $states = [];
        $observed = [];
        foreach ($evidence as $item) {
            $states[$item->state] = true;
            if ($item->observedAt !== null) {
                $observed[] = $item->observedAt;
            }
        }
        $observedAt = $observed === [] ? null : min($observed);

        if (count($states) !== 1) {
            return [PublicWorldStatus::RUNTIME_DEGRADED, $observedAt];
        }

        $state = array_key_first($states);
        if ($state === null) {
            return [PublicWorldStatus::RUNTIME_UNAVAILABLE, $observedAt];
        }
        if ($state !== NativeRuntimeStatusReadModel::FRESH) {
            return [match ($state) {
                NativeRuntimeStatusReadModel::STALE => PublicWorldStatus::RUNTIME_STALE,
                NativeRuntimeStatusReadModel::INVALID => PublicWorldStatus::RUNTIME_INVALID,
                default => PublicWorldStatus::RUNTIME_UNAVAILABLE,
            }, $observedAt];
        }

        $ready = [];
        foreach ($evidence as $item) {
            if ($item->ready === null) {
                return [PublicWorldStatus::RUNTIME_DEGRADED, $observedAt];
            }
            $ready[$item->ready ? 'ready' : 'not_ready'] = true;
        }
        if (count($ready) !== 1) {
            return [PublicWorldStatus::RUNTIME_DEGRADED, $observedAt];
        }

        return [
            isset($ready['ready']) ? PublicWorldStatus::RUNTIME_READY : PublicWorldStatus::RUNTIME_NOT_READY,
            $observedAt,
        ];
    }
}
