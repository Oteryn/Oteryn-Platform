<?php

namespace App\PublicGameData;

use App\CharacterProfiles\CharacterGuildVisibility;
use App\GameProjections\PublicProjectionFamily;
use App\GameProjections\PublicProjectionReadModel;
use stdClass;

/**
 * Native reader of the online and highscores pages (removal plan packets C5 and C7) over the
 * Game Public Projections v1 read model. It is the only source of these pages: a stale or missing
 * family, an unresolved World or an absent list raises {@see PublicGameDataUnavailable}; nothing
 * is ever invented or served from another source.
 */
final class NativePublicGameData
{
    private const UUID = '/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/D';

    public function __construct(
        private readonly PublicProjectionReadModel $readModel,
        private readonly CharacterGuildVisibility $guildVisibility,
    ) {}

    /**
     * Coarse online presence (name, level, vocation), sorted by name; may be empty.
     *
     * @return list<stdClass>
     *
     * @throws PublicGameDataUnavailable
     */
    public function online(int $now): array
    {
        $world = $this->world(PublicProjectionFamily::WorldOnline, $now);
        $characters = $this->entry(PublicProjectionFamily::WorldOnline, $world, $now)['characters'] ?? null;
        if (! is_array($characters) || ! array_is_list($characters)) {
            throw new PublicGameDataUnavailable('Online list is not available.');
        }

        $online = [];
        foreach ($characters as $character) {
            if (! is_array($character) || ! is_string($character['name'] ?? null)) {
                throw new PublicGameDataUnavailable('Online list is not available.');
            }
            $online[] = $this->object([
                'name' => $character['name'],
                'level' => $this->count($character['level'] ?? null),
                'vocation' => is_int($character['vocation'] ?? null) ? $character['vocation'] : 0,
            ]);
        }

        return $online;
    }

    /**
     * Published rows of (World, category, filter) in rank order, each joined to its current
     * `character_profile` entry in the same World and shown under the profile's name. A row
     * without a servable profile is left out and the other rows keep their published rank.
     *
     * @return list<stdClass> rank, name, score, level, vocation
     *
     * @throws PublicGameDataUnavailable
     */
    public function highscores(string $category, ?int $vocation, int $now): array
    {
        $world = $this->world(PublicProjectionFamily::Highscores, $now);
        $filter = $vocation === null ? 0 : CommunityDataPolicy::highscoreVocationFilter($vocation);
        $subject = $world.':'.CommunityDataPolicy::highscoreCategoryId($category).':'.$filter;

        $rows = $this->entry(PublicProjectionFamily::Highscores, $subject, $now)['rows'] ?? null;
        if (! is_array($rows) || ! array_is_list($rows)) {
            throw new PublicGameDataUnavailable('Highscores are not available.');
        }

        $ids = [];
        foreach ($rows as $row) {
            if (is_array($row) && is_string($row['character_id'] ?? null)) {
                $ids[] = $row['character_id'];
            }
        }
        // A stale profile family serves no highscores row at all (§4.1).
        $profiles = $this->readModel->entries(PublicProjectionFamily::CharacterProfile, $ids, $now)
            ?? throw new PublicGameDataUnavailable('Character profiles are not available.');

        $players = [];
        foreach ($rows as $row) {
            if (! is_array($row) || ! is_string($row['character_id'] ?? null)) {
                throw new PublicGameDataUnavailable('Highscores are not available.');
            }
            $entries = $profiles[$row['character_id']]['profile'] ?? null;
            $profile = is_array($entries) ? ($entries[0] ?? null) : null;
            if (! is_array($profile) || ($profile['world_id'] ?? null) !== $world || ! is_string($profile['name'] ?? null)) {
                continue;
            }
            $rowVocation = is_int($row['vocation'] ?? null) ? $row['vocation'] : 0;

            $players[] = $this->object([
                'rank' => $this->count($row['rank'] ?? null),
                'name' => $profile['name'],
                'score' => $this->count($category === 'level' ? ($row['level'] ?? null) : ($row['value'] ?? null)),
                'level' => $this->count($row['level'] ?? null),
                'vocation' => $rowVocation,
            ]);
        }

        return $players;
    }

    /**
     * The public profile of a character by name in the served World, with exactly the fields
     * `character_profile` carries (name, level, vocation, guild membership shown only
     * when `guild_visible`). The guild and rank
     * names come from the `guild` family and are left out when that family cannot name them.
     * Null when no live character of that name is published.
     *
     * @return stdClass|null name, level, vocation, guild_visible, guild_member, guild_name, guild_rank
     *
     * @throws PublicGameDataUnavailable
     */
    public function profile(string $name, int $now): ?stdClass
    {
        $world = $this->world(PublicProjectionFamily::CharacterProfile, $now);
        $found = $this->readModel->entryByName(PublicProjectionFamily::CharacterProfile, $name, $now);
        $payload = $found['payload'] ?? [];
        $entries = $payload['profile'] ?? null;
        $profile = is_array($entries) ? ($entries[0] ?? null) : null;
        // The one case-folded holder must live in the served World; the family may have gone stale
        // while the lookup ran.
        if ($found === null || $found['world_id'] !== $world || ! is_array($profile)
            || ($profile['world_id'] ?? null) !== $world || ! is_string($profile['name'] ?? null)) {
            if ($this->readModel->hasWorld(PublicProjectionFamily::CharacterProfile, $world, $now) !== true) {
                throw new PublicGameDataUnavailable('Public projection World is not available.');
            }

            return null;
        }

        $guildName = null;
        $guildRank = null;
        $memberships = $profile['guild'] ?? null;
        $membership = is_array($memberships) ? ($memberships[0] ?? null) : null;
        // `guild_visible` is the owner's choice carried by Game: when it is not exactly true the
        // page learns nothing about the membership, not even that one exists.
        // Game's flag and the owner's Platform preference must both allow the guild row, including
        // the "no guild" row: an opt-out hides membership status, not only the guild name.
        $characterId = is_string($payload['character_id'] ?? null) ? $payload['character_id'] : '';
        $guildVisible = ($profile['guild_visible'] ?? false) === true
            && $this->guildVisibility->allows($characterId);
        if (! $guildVisible) {
            $membership = null;
        }
        if (is_array($membership) && is_string($membership['guild_id'] ?? null)) {
            $guildId = $membership['guild_id'];
            $guildEntries = $this->readModel->entries(PublicProjectionFamily::Guild, [$guildId], $now)[$guildId]['guild'] ?? null;
            $guild = is_array($guildEntries) ? ($guildEntries[0] ?? null) : null;
            if (is_array($guild) && ($guild['world_id'] ?? null) === $world && is_string($guild['name'] ?? null)) {
                $guildName = $guild['name'];
                foreach (is_array($guild['ranks'] ?? null) ? $guild['ranks'] : [] as $rank) {
                    if (is_array($rank) && ($rank['level'] ?? null) === ($membership['rank_level'] ?? false) && is_string($rank['name'] ?? null)) {
                        $guildRank = $rank['name'];
                    }
                }
            }
        }

        return $this->object([
            'name' => $profile['name'],
            'level' => $this->count($profile['level'] ?? null),
            'vocation' => is_int($profile['vocation'] ?? null) ? $profile['vocation'] : 0,
            'guild_visible' => $guildVisible,
            'guild_member' => is_array($membership),
            'guild_name' => $guildName,
            'guild_rank' => $guildRank,
        ]);
    }

    /**
     * @return array<string, mixed>
     *
     * @throws PublicGameDataUnavailable when the family is stale or the subject was never published
     */
    private function entry(PublicProjectionFamily $family, string $subject, int $now): array
    {
        return $this->readModel->entries($family, [$subject], $now)[$subject] ?? throw new PublicGameDataUnavailable('Public projection entry is not available.');
    }

    /**
     * The configured World, or the only World that has published the family.
     *
     * @throws PublicGameDataUnavailable
     */
    private function world(PublicProjectionFamily $family, int $now): string
    {
        $configured = config('public_game_data.world_id');
        if (is_string($configured) && trim($configured) !== '') {
            $world = strtolower(trim($configured));
            if (preg_match(self::UUID, $world) !== 1) {
                throw new PublicGameDataUnavailable('Configured World is invalid.');
            }
            // Keep the freshness check even when the World is configured.
            if (! $this->readModel->fresh($family, $now)) {
                throw new PublicGameDataUnavailable('Public projection is stale.');
            }

            return $world;
        }

        $worlds = $this->readModel->worlds($family, $now);
        if ($worlds === null || count($worlds) !== 1) {
            throw new PublicGameDataUnavailable('Public projection World is not resolved.');
        }

        return $worlds[0];
    }

    /** Canonical uint64 string to a non-negative int, saturating at PHP_INT_MAX. */
    private function count(mixed $value): int
    {
        if (! is_string($value) || preg_match('/^(0|[1-9][0-9]*)$/D', $value) !== 1) {
            return 0;
        }

        $max = (string) PHP_INT_MAX;
        if (strlen($value) > strlen($max) || (strlen($value) === strlen($max) && strcmp($value, $max) > 0)) {
            return PHP_INT_MAX;
        }

        return (int) $value;
    }

    /** @param array<string, mixed> $values */
    private function object(array $values): stdClass
    {
        return (object) $values;
    }
}
