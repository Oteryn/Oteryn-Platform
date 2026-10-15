<?php

namespace App\PublicGameData;

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

    public function __construct(private readonly PublicProjectionReadModel $readModel) {}

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
            // A promoted vocation selects its base list; keep only that exact vocation.
            if ($vocation !== null && $rowVocation !== $vocation) {
                continue;
            }

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
            if ($this->readModel->worlds($family, $now) === null) {
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

        return strlen($value) > 18 ? PHP_INT_MAX : (int) $value;
    }

    /** @param array<string, mixed> $values */
    private function object(array $values): stdClass
    {
        return (object) $values;
    }
}
