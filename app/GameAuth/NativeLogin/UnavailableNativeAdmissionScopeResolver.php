<?php

namespace App\GameAuth\NativeLogin;

/**
 * Default resolver until the Character read model and native route selection exist (N4P-3):
 * no route is ever selectable, so issuance fails closed and the ticket stays unused.
 */
final class UnavailableNativeAdmissionScopeResolver implements NativeAdmissionScopeResolver
{
    public function resolve(RedeemedNativeAccount $account, NativeAdmissionRequest $request): NativeAdmissionScope
    {
        throw new NativeLoginRefused(NativeLoginError::RouteUnavailable);
    }
}
