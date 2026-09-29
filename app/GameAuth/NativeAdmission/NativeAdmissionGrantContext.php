<?php

namespace App\GameAuth\NativeAdmission;

use App\Identity\Support\CanonicalAccountId;
use InvalidArgumentException;

/**
 * Validated per-attempt facts bound into an oteryn-pre-admission-v1 grant
 * (OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT §8.2). Constant claims, iat/nbf/exp and
 * jti are added by the issuer.
 */
final readonly class NativeAdmissionGrantContext
{
    // Conservative subset of the FND-04 §5 token grammar; the pinned Game verifier fixtures (§16) are authoritative.
    private const REVISION = '/^[A-Za-z0-9][A-Za-z0-9._:-]{0,63}$/';

    private const NON_ZERO_UINT64 = '/^[1-9][0-9]{0,19}$/';

    public function __construct(
        public string $attemptRef,
        public string $accountId,
        public string $characterId,
        public string $worldId,
        public string $channelId,
        public string $accountSecurityGeneration,
        public string $routeRevision,
        public string $runtimeObservationRevision,
        public string $scopeOwnershipGeneration,
        public string $rulesetRevision,
        public string $contentRevision,
        public string $mapRevision,
        public string $worldPolicyRevision,
        public string $offerRevision,
    ) {
        foreach ([
            'attempt_ref' => $attemptRef,
            'account_id' => $accountId,
            'character_id' => $characterId,
            'world_id' => $worldId,
            'channel_id' => $channelId,
        ] as $name => $value) {
            if (! CanonicalAccountId::isValid($value)) {
                throw new InvalidArgumentException("Native admission {$name} is not a canonical UUIDv7.");
            }
        }

        foreach ([
            'account_security_generation' => $accountSecurityGeneration,
            'scope_ownership_generation' => $scopeOwnershipGeneration,
        ] as $name => $value) {
            if (preg_match(self::NON_ZERO_UINT64, $value) !== 1
                || (strlen($value) === 20 && strcmp($value, '18446744073709551615') > 0)) {
                throw new InvalidArgumentException("Native admission {$name} is not a non-zero uint64 decimal.");
            }
        }

        foreach ([
            'route_revision' => $routeRevision,
            'runtime_observation_revision' => $runtimeObservationRevision,
            'ruleset_revision' => $rulesetRevision,
            'content_revision' => $contentRevision,
            'map_revision' => $mapRevision,
            'world_policy_revision' => $worldPolicyRevision,
            'offer_revision' => $offerRevision,
        ] as $name => $value) {
            if (preg_match(self::REVISION, $value) !== 1) {
                throw new InvalidArgumentException("Native admission {$name} violates the revision grammar.");
            }
        }
    }
}
