<?php

namespace App\GameAuth\NativeLogin;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReadModel;
use App\GameAuth\Worlds\NativeRouteRecord;
use App\GameAuth\Worlds\NativeRouteRecords;
use UnexpectedValueException;

/**
 * Character check (§5.4) and native route selection (§7.4) over the Registry route records and the
 * fresh, ownership-bound runtime status read model.
 *
 * No Character read model exists yet (Q17a). D171 allows issuance without verifying AccountId ->
 * CharacterId ownership only in `testing`/`preproduction`, only while
 * `game-auth.native_admission.unverified_character_ownership` is on, and only for the configured
 * `unverified_character_world_id`; Game FND-04A §5 admission stays the fail-closed guard (it rejects a
 * foreign or moved Character with ADMISSION_ACCOUNT_CHARACTER_CONFLICT / ADMISSION_GRANT_WORLD_STALE).
 * With the mode off there is no source of Character facts, so issuance fails closed (ROUTE_UNAVAILABLE,
 * §5.4 "account read model unavailable"); with the mode on outside those environments, UNAVAILABLE.
 * Release gate: CHAR-NAME-1 -> LCFA-1 and the full §5.4 check replace this mode.
 */
final class RegistryNativeAdmissionScopeResolver implements NativeAdmissionScopeResolver
{
    private const UUID_V7 = '/\A[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/';

    public function __construct(
        private readonly NativeRouteRecords $routes,
        private readonly NativeRuntimeStatusReadModel $runtime,
    ) {}

    public function resolve(RedeemedNativeAccount $account, NativeAdmissionRequest $request): NativeAdmissionScope
    {
        $worldId = self::unverifiedCharacterWorld();

        // Shared epoch lock (lock order: epoch, then nothing else here): an epoch raise cannot land between
        // the ownership check of the selected report and signing. A waiter is bounded by the issuer's lock
        // wait timeout and fails closed as NATIVE_LOGIN_UNAVAILABLE.
        try {
            $this->runtime->lockEpoch(exclusive: false);
            $now = now()->getTimestamp();
            foreach ($this->routes->forWorld($worldId) as $route) {
                if ($request->channelId !== null && $route->channelId !== $request->channelId) {
                    continue;
                }
                $scope = $this->candidate($route, $request->characterId, $now);
                if ($scope !== null) {
                    return $scope;
                }
            }
        } catch (UnexpectedValueException) {
            throw new NativeLoginRefused(NativeLoginError::Unavailable);
        }

        throw new NativeLoginRefused(NativeLoginError::RouteUnavailable);
    }

    /** §7.4 conditions 3 to 5 for one login-enabled Registry route (conditions 1 and 2). */
    private function candidate(NativeRouteRecord $route, string $characterId, int $now): ?NativeAdmissionScope
    {
        $report = $this->runtime->routable($route->worldId, $route->channelId, $now);
        if ($report === null
            || ! hash_equals($route->routeRevision, $report->routeRevision)
            || $report->protocolMajor !== NativeRouteRecord::PROTOCOL_MAJOR
            || $report->transportProfile !== NativeRouteRecord::TRANSPORT_PROFILE) {
            return null;
        }

        // The request's offer was already required to contain (1, 1, oteryn-game/1), the only route transport.
        return new NativeAdmissionScope(
            characterId: $characterId,
            worldId: $route->worldId,
            channelId: $route->channelId,
            routeRevision: $route->routeRevision,
            runtimeObservationRevision: $report->runtimeObservationRevision,
            scopeOwnershipGeneration: $report->scopeOwnershipGeneration,
            rulesetRevision: $report->rulesetRevision,
            contentRevision: $report->contentRevision,
            mapRevision: $report->mapRevision,
            worldPolicyRevision: $report->worldPolicyRevision,
            offerRevision: $report->offerRevision,
        );
    }

    /** D171 gate; the Character's world in the unverified testing mode. */
    private static function unverifiedCharacterWorld(): string
    {
        if (filter_var(config('game-auth.native_admission.unverified_character_ownership'), FILTER_VALIDATE_BOOL) !== true) {
            throw new NativeLoginRefused(NativeLoginError::RouteUnavailable);
        }
        $worldId = config('game-auth.native_admission.unverified_character_world_id');
        if (! app()->environment(['testing', 'preproduction'])
            || ! is_string($worldId) || preg_match(self::UUID_V7, $worldId) !== 1) {
            throw new NativeLoginRefused(NativeLoginError::Unavailable);
        }

        return $worldId;
    }
}
