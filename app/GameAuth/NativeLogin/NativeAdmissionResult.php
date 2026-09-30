<?php

namespace App\GameAuth\NativeLogin;

use SensitiveParameter;

/**
 * Committed native admission for the §3.2 response. The caller adds the endpoint from the World
 * Registry route record whose route_revision is bound here.
 */
final readonly class NativeAdmissionResult
{
    public function __construct(
        public string $attemptRef,
        public string $worldId,
        public string $channelId,
        public string $routeRevision,
        #[SensitiveParameter] public string $token,
        public int $validForSeconds,
    ) {}

    /** @return array<string, mixed> */
    public function __debugInfo(): array
    {
        return [
            'attemptRef' => $this->attemptRef,
            'worldId' => $this->worldId,
            'channelId' => $this->channelId,
            'validForSeconds' => $this->validForSeconds,
        ];
    }
}
