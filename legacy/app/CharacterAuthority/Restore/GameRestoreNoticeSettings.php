<?php

namespace App\CharacterAuthority\Restore;

use App\CharacterAuthority\CharacterCommandSettings;
use App\CharacterAuthority\CharacterCommandWire;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;

/**
 * Game restore notice configuration. Off by default and, unlike the command surface, not limited
 * to an environment: Game opens admission after a restore only on Platform's acknowledgement, so
 * a stack whose Game can be restored must be able to answer. The dedicated identity list shares no
 * subject with any other internal purpose, the character-command identities included.
 */
final readonly class GameRestoreNoticeSettings
{
    /** @param list<string> $identities */
    private function __construct(
        private array $identities,
        public string $sourceAuthority,
        public int $requestsPerMinute,
    ) {}

    public static function current(): ?self
    {
        if (config('character-authority.restore.enabled') !== true) {
            return null;
        }

        $identities = CharacterCommandSettings::dedicatedIdentities(
            config('character-authority.restore.identities'),
            CharacterCommandSettings::configuredIdentities(),
        );
        $sourceAuthority = config('character-authority.restore.source_authority');
        $rate = NativeRuntimeStatusSettings::bounded(config('character-authority.restore.requests_per_minute'), 1, 600);
        if ($identities === null || ! CharacterCommandWire::isSourceAuthority($sourceAuthority) || $rate === null) {
            return null;
        }
        /** @var string $sourceAuthority */

        return new self($identities, $sourceAuthority, $rate);
    }

    /** @return list<string> the configured subjects, valid or not, for the overlap checks of other purposes */
    public static function configuredIdentities(): array
    {
        return CharacterCommandSettings::listedStrings(config('character-authority.restore.identities'));
    }

    public function knows(string $identity): bool
    {
        return in_array($identity, $this->identities, true);
    }
}
