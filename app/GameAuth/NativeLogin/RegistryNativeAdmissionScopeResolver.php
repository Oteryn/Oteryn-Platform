<?php

namespace App\GameAuth\NativeLogin;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersAccountView;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReadModel;
use App\GameAuth\Worlds\NativeRouteRecord;
use App\GameAuth\Worlds\NativeRouteRecords;
use UnexpectedValueException;

/**
 * Character check (§5.4) and native route selection (§7.4) over Platform's read-only Game projections.
 *
 * When the LCFA consumer is enabled, its live highest-epoch account view is mandatory and D171 is never
 * used as a fallback. D171 remains testing/preproduction-only while LCFA is explicitly disabled.
 */
final class RegistryNativeAdmissionScopeResolver implements NativeAdmissionScopeResolver
{
    private const UUID_V7 = '/\A[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/';

    public function __construct(
        private readonly NativeRouteRecords $routes,
        private readonly NativeRuntimeStatusReadModel $runtime,
        private readonly NativeAccountCharactersReadModel $characters,
    ) {}

    public function resolve(RedeemedNativeAccount $account, NativeAdmissionRequest $request): NativeAdmissionScope
    {
        // The caller (NativeAdmissionAttempts::issue) already holds the runtime shared epoch lock before
        // any non-locking issuer read. LCFA then takes its own shared state->snapshot locks in the same
        // order as LCFA ingestion, so a projection epoch raise cannot split this ownership check.
        try {
            $now = now()->getTimestamp();
            $worldId = $this->characterWorld($account, $request, $now);

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

    private function characterWorld(RedeemedNativeAccount $account, NativeAdmissionRequest $request, int $now): string
    {
        if (filter_var(config('game-auth.native_account_characters.enabled'), FILTER_VALIDATE_BOOL) !== true) {
            return self::unverifiedCharacterWorld();
        }

        $view = $this->characters->viewForAccount($account->accountId, $now, lock: true);
        if ($view->state === NativeAccountCharactersAccountView::UNAVAILABLE) {
            // Enabled outside testing/preproduction, or misconfigured: a Platform fault, not a route state.
            throw new NativeLoginRefused(NativeLoginError::Unavailable);
        }
        if ($view->state === NativeAccountCharactersAccountView::MISSING) {
            throw new NativeLoginRefused(NativeLoginError::CharacterConflict);
        }
        if ($view->state !== NativeAccountCharactersAccountView::READY) {
            throw new NativeLoginRefused(NativeLoginError::RouteUnavailable);
        }

        foreach ($view->characters as $character) {
            if (! hash_equals($character->characterId, $request->characterId)) {
                continue;
            }
            if ($character->availability !== 'AVAILABLE') {
                throw new NativeLoginRefused(NativeLoginError::CharacterConflict);
            }

            return $character->worldId;
        }

        throw new NativeLoginRefused(NativeLoginError::CharacterConflict);
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

    /** D171 gate; used only while LCFA is explicitly disabled. */
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
