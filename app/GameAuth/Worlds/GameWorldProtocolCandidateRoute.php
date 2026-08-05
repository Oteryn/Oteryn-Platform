<?php

namespace App\GameAuth\Worlds;

final readonly class GameWorldProtocolCandidateRoute
{
    /**
     * @param  list<string>  $requiredCapabilities
     * @param  list<string>  $optionalCapabilities
     */
    public function __construct(
        public string $family,
        public int $nativeProtocolVersion,
        public string $transport,
        public int $schemaRevision,
        public string $schemaSha256,
        public array $requiredCapabilities,
        public array $optionalCapabilities,
        public string $endpointId,
        public string $host,
        public int $port,
        public string $tlsServerName,
    ) {}
}
