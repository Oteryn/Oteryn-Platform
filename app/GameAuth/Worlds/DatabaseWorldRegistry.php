<?php

namespace App\GameAuth\Worlds;

final class DatabaseWorldRegistry implements WorldRegistry
{
    private const IDENTIFIER_PATTERN = '/\A[a-z0-9][a-z0-9._-]{0,63}\z/D';

    /**
     * @return list<GameWorldRoute>
     */
    public function forAccount(int $canaryAccountId): array
    {
        if ($canaryAccountId < 1) {
            return [];
        }

        $routes = [];
        $worlds = GameWorld::query()
            ->where('login_enabled', true)
            ->where('status', GameWorldStatus::Online->value)
            ->orderBy('id')
            ->get();

        foreach ($worlds as $world) {
            if (! $this->isRoutableHostAndPort($world->game_host, $world->game_port)) {
                continue;
            }

            if ($world->slug === '' || $world->name === '' || $world->region === '') {
                continue;
            }

            $routes[] = new GameWorldRoute(
                id: $world->id,
                slug: $world->slug,
                name: $world->name,
                region: $world->region,
                host: $world->game_host,
                port: $world->game_port,
                gameplayPolicy: $this->gameplayPolicy($world),
            );
        }

        return $routes;
    }

    private function gameplayPolicy(GameWorld $world): GameWorldProtocolPolicy
    {
        $projected = [];
        $candidates = $world->protocolCandidates()
            ->where('enabled', true)
            ->where('channel_id', 1)
            ->orderBy('sort_order')
            ->orderBy('id')
            ->get();

        foreach ($candidates as $candidate) {
            $route = $this->projectCandidate($candidate);
            if ($route !== null) {
                $projected[] = $route;
            }
        }

        return new GameWorldProtocolPolicy(
            revision: $world->gameplay_policy_revision,
            channelId: 1,
            candidates: $projected,
        );
    }

    private function projectCandidate(GameWorldProtocolCandidate $candidate): ?GameWorldProtocolCandidateRoute
    {
        if ($candidate->channel_id !== 1
            || $candidate->sort_order < 0
            || $candidate->schema_revision < 1
            || ! $this->isIdentifier($candidate->family)
            || ! $this->isIdentifier($candidate->profile)
            || ! $this->isIdentifier($candidate->transport)
            || ! $this->isIdentifier($candidate->endpoint_id)
            || preg_match('/\A[0-9a-f]{64}\z/D', $candidate->schema_sha256) !== 1
            || ! $this->isRoutableHostAndPort($candidate->game_host, $candidate->game_port)
            || ! $this->isTlsServerName($candidate->tls_server_name)
        ) {
            return null;
        }

        $required = $this->canonicalCapabilities($candidate->required_capabilities);
        $optional = $this->canonicalCapabilities($candidate->optional_capabilities);
        if ($required === null || $optional === null || array_intersect($required, $optional) !== []) {
            return null;
        }

        return new GameWorldProtocolCandidateRoute(
            family: $candidate->family,
            profile: $candidate->profile,
            transport: $candidate->transport,
            schemaRevision: $candidate->schema_revision,
            schemaSha256: $candidate->schema_sha256,
            requiredCapabilities: $required,
            optionalCapabilities: $optional,
            endpointId: $candidate->endpoint_id,
            host: $candidate->game_host,
            port: $candidate->game_port,
            tlsServerName: $candidate->tls_server_name,
        );
    }

    /**
     * @param  mixed  $capabilities
     * @return list<string>|null
     */
    private function canonicalCapabilities(mixed $capabilities): ?array
    {
        if (! is_array($capabilities) || count($capabilities) > 64) {
            return null;
        }

        $normalized = [];
        foreach ($capabilities as $capability) {
            if (! is_string($capability) || ! $this->isIdentifier($capability)) {
                return null;
            }
            $normalized[] = $capability;
        }

        $sorted = $normalized;
        sort($sorted, SORT_STRING);
        if ($normalized !== $sorted || count(array_unique($normalized)) !== count($normalized)) {
            return null;
        }

        return array_values($normalized);
    }

    private function isIdentifier(string $value): bool
    {
        return preg_match(self::IDENTIFIER_PATTERN, $value) === 1;
    }

    private function isRoutableHostAndPort(string $host, int $port): bool
    {
        $host = trim($host);
        $validHost = filter_var($host, FILTER_VALIDATE_IP) !== false
            || filter_var($host, FILTER_VALIDATE_DOMAIN, FILTER_FLAG_HOSTNAME) !== false;

        return $host !== '' && $validHost && $port >= 1 && $port <= 65535;
    }

    private function isTlsServerName(string $serverName): bool
    {
        if ($serverName === '' || strlen($serverName) > 253 || str_contains($serverName, '*')) {
            return false;
        }

        return filter_var($serverName, FILTER_VALIDATE_IP) !== false
            || filter_var($serverName, FILTER_VALIDATE_DOMAIN, FILTER_FLAG_HOSTNAME) !== false;
    }
}
