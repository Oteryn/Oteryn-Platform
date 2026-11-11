<?php

namespace App\GameAuth\NativeLogin;

/**
 * Character check (§5.4) and native route selection (§7.4) inside the issuer transaction.
 * Implementations throw NativeLoginRefused with CHARACTER_CONFLICT, ROUTE_UNAVAILABLE or
 * OFFER_UNSUPPORTED; they must return the requested character_id only after validating it.
 */
interface NativeAdmissionScopeResolver
{
    public function resolve(RedeemedNativeAccount $account, NativeAdmissionRequest $request): NativeAdmissionScope;
}
