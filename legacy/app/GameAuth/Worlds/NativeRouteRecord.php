<?php

namespace App\GameAuth\Worlds;

use InvalidArgumentException;

/**
 * One World Registry route record for a Registry-issued (WorldId, ChannelId) (login contract §7.3,
 * Decision D3). The route_revision digest binds the version to exactly one scope and one endpoint, so
 * a revision copied from another scope or endpoint never matches. D172: testing/preproduction only;
 * the client trusts the gameplay certificate through the configured `oteryn-dev-client` root until U3.
 */
final readonly class NativeRouteRecord
{
    public const ALPN = 'oteryn-game/1';

    public const PROTOCOL_MAJOR = 1;

    public const TRANSPORT_PROFILE = 1;

    private const UUID_V7 = '/\A[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/';

    private const DNS_NAME = '/\A(?=.{1,253}\z)(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)(?:\.(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?))*\z/';

    public string $routeRevision;

    public function __construct(
        public string $worldId,
        public string $channelId,
        public string $host,
        public int $port,
        public string $tlsServerName,
        public int $version,
    ) {
        if (preg_match(self::UUID_V7, $worldId) !== 1 || preg_match(self::UUID_V7, $channelId) !== 1
            || ! self::validHost($host)
            || $port < 1 || $port > 65535
            || ! self::validServerName($tlsServerName)
            || $version < 1 || $version > 4_294_967_295) {
            throw new InvalidArgumentException('Native route record is invalid.');
        }

        $this->routeRevision = 'rt.'.$version.'.'.substr(hash('sha256', $this->canonicalDescriptor()), 0, 32);
    }

    /** RFC 8785 serialization of the §7.3 route descriptor: members sorted by name, ASCII only. */
    public function canonicalDescriptor(): string
    {
        return json_encode([
            'alpn' => self::ALPN,
            'channel_id' => $this->channelId,
            'host' => $this->host,
            'port' => $this->port,
            'protocol_major' => self::PROTOCOL_MAJOR,
            'tls_server_name' => $this->tlsServerName,
            'transport_profile' => self::TRANSPORT_PROFILE,
            'world_id' => $this->worldId,
        ], JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
    }

    public function sameEndpoint(string $host, int $port, string $tlsServerName): bool
    {
        return $this->host === $host && $this->port === $port && $this->tlsServerName === $tlsServerName;
    }

    /**
     * The §3.2 `endpoint` member, in contract order.
     *
     * @return array{host: string, port: int, tls_server_name: string, alpn: string, protocol_major: int, transport_profile: int} */
    public function endpoint(): array
    {
        return [
            'host' => $this->host,
            'port' => $this->port,
            'tls_server_name' => $this->tlsServerName,
            'alpn' => self::ALPN,
            'protocol_major' => self::PROTOCOL_MAJOR,
            'transport_profile' => self::TRANSPORT_PROFILE,
        ];
    }

    /** A lowercase LDH DNS name whose last label is not all digits, so it can never read as an IP address. */
    private static function validServerName(string $name): bool
    {
        return preg_match(self::DNS_NAME, $name) === 1 && preg_match('/(?:\A|\.)[0-9]+\z/', $name) !== 1;
    }

    /** A lowercase LDH DNS name or a canonical IPv4/IPv6 literal (§3.2 `endpoint.host`). */
    private static function validHost(string $host): bool
    {
        if (preg_match(self::DNS_NAME, $host) === 1) {
            return true;
        }
        $ip = filter_var($host, FILTER_VALIDATE_IP);

        return is_string($ip) && inet_ntop((string) inet_pton($ip)) === $host;
    }
}
