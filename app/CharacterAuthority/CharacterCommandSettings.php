<?php

namespace App\CharacterAuthority;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentReport;
use JsonException;

/**
 * Character Authority command configuration. Off by default and limited to testing/preproduction
 * (Game contract §14: Platform stays disabled until Game supports this version, D963 Q2=A). Any
 * identity shared with another internal purpose makes the whole surface unavailable (§3).
 */
final readonly class CharacterCommandSettings
{
    /** @param list<string> $identities */
    private function __construct(
        private array $identities,
        public string $sourceAuthority,
        public int $intentTtlSeconds,
        public int $maxPendingIntents,
        public int $requestsPerMinute,
    ) {}

    public static function current(): ?self
    {
        if (config('character-authority.commands.enabled') !== true || ! app()->environment(['testing', 'preproduction'])) {
            return null;
        }

        $identities = self::identities(config('character-authority.commands.identities'));
        $sourceAuthority = config('character-authority.commands.source_authority');
        $ttl = NativeRuntimeStatusSettings::bounded(config('character-authority.commands.intent_ttl_seconds'), 1, CharacterCommandWire::MAX_TTL_SECONDS);
        $backlog = NativeRuntimeStatusSettings::bounded(config('character-authority.commands.max_pending_intents'), 1, CharacterCommandWire::MAX_PENDING_BACKLOG);
        $rate = NativeRuntimeStatusSettings::bounded(config('character-authority.commands.requests_per_minute'), 1, 6000);
        if ($identities === null || ! is_string($sourceAuthority) || ! CharacterCommandWire::isSourceAuthority($sourceAuthority)
            || $ttl === null || $backlog === null || $rate === null) {
            return null;
        }

        return new self($identities, $sourceAuthority, $ttl, $backlog, $rate);
    }

    public function knows(string $identity): bool
    {
        return in_array($identity, $this->identities, true);
    }

    /** @return list<string>|null */
    private static function identities(mixed $raw): ?array
    {
        $decoded = self::decoded($raw);
        if (! is_array($decoded) || $decoded === [] || ! array_is_list($decoded)) {
            return null;
        }

        $otherPurposes = [
            config('game-auth.native_evidence.mtls_client_identity'),
            config('game-auth.character_bootstrap_intent.mtls_client_identity'),
            config('products-entitlements.premium_snapshot.mtls_client_identity'),
            ...NativeAccountCharactersSettings::configuredPublisherIdentities(),
        ];
        foreach (['native_runtime_status', 'native_scope_assignment'] as $section) {
            $other = self::decoded(config('game-auth.'.$section.'.identities'));
            if (is_array($other) && ! array_is_list($other)) {
                array_push($otherPurposes, ...array_map('strval', array_keys($other)));
            }
        }

        $identities = [];
        foreach ($decoded as $identity) {
            if (! is_string($identity)
                || preg_match(NativeScopeAssignmentReport::IDENTITY, $identity) !== 1
                || in_array($identity, $otherPurposes, true)
                || in_array($identity, $identities, true)) {
                return null;
            }
            $identities[] = $identity;
        }

        return $identities;
    }

    private static function decoded(mixed $raw): mixed
    {
        try {
            return is_string($raw) ? json_decode($raw, true, 3, JSON_THROW_ON_ERROR) : $raw;
        } catch (JsonException) {
            return null;
        }
    }
}
