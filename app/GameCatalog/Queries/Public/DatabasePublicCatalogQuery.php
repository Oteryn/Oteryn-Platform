<?php

namespace App\GameCatalog\Queries\Public;

use App\GameCatalog\Application\PublicRead\PublicCatalogContext;
use App\GameCatalog\Application\PublicRead\PublicCatalogContextResolver;
use App\GameCatalog\Infrastructure\Persistence\CatalogDatabaseRow;
use Illuminate\Database\Query\Builder;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Support\Facades\DB;
use JsonException;
use RuntimeException;

final readonly class DatabasePublicCatalogQuery
{
    private const PER_PAGE = 24;

    public function __construct(private PublicCatalogContextResolver $contexts) {}

    public function summary(): ?PublicCatalogSummary
    {
        $context = $this->contexts->resolve();
        if ($context === null) {
            return null;
        }

        return new PublicCatalogSummary(
            context: $context,
            itemCount: $this->baseItemQuery($context, 'en')->count(),
            creatureCount: $this->baseCreatureQuery($context, 'en')->count(),
        );
    }

    public function items(
        string $locale,
        string $query,
        ?string $category,
        ?string $weaponType,
        int $page,
    ): ?PublicCatalogItemPage {
        $context = $this->contexts->resolve();
        if ($context === null) {
            return null;
        }

        $base = $this->baseItemQuery($context, $locale);
        $categories = $this->distinctStrings(clone $base, 'item.category');
        $weaponTypes = $this->distinctStrings(clone $base, 'item.weapon_type');
        $filtered = clone $base;

        if ($query !== '') {
            $like = '%'.$this->escapeLike($query).'%';
            $filtered->where(function (Builder $builder) use ($like): void {
                $builder->where('item.name', 'like', $like)
                    ->orWhere('translation.display_name', 'like', $like);
            });
        }
        if ($category !== null) {
            $filtered->where('item.category', $category);
        }
        if ($weaponType !== null) {
            $filtered->where('item.weapon_type', $weaponType);
        }

        $total = $filtered->count();
        $rows = $filtered
            ->orderByRaw('COALESCE(translation.display_name, item.name) ASC')
            ->orderBy('entity.canonical_key')
            ->forPage($page, self::PER_PAGE)
            ->get([
                'entity.canonical_key',
                'translation.slug as translation_slug',
                'translation.display_name as translated_name',
                'translation.summary as translated_summary',
                'item.name',
                'item.description',
                'item.category',
                'item.weapon_type',
                'item.attack',
                'item.defense',
                'item.armor',
                'item.minimum_level',
                'item.vocations',
                'item.image_key',
            ]);

        $items = [];
        foreach ($rows as $row) {
            $databaseRow = CatalogDatabaseRow::from($row);
            $items[] = new PublicCatalogItemCard(
                slug: $databaseRow->nullableString('translation_slug') ?? $this->slugFromCanonicalKey($databaseRow->string('canonical_key'), 'item'),
                name: $databaseRow->nullableString('translated_name') ?? $databaseRow->string('name'),
                summary: $databaseRow->nullableString('translated_summary') ?? $databaseRow->nullableString('description'),
                category: $databaseRow->string('category'),
                weaponType: $databaseRow->nullableString('weapon_type'),
                attack: $databaseRow->nullableInt('attack'),
                defense: $databaseRow->nullableInt('defense'),
                armor: $databaseRow->nullableInt('armor'),
                minimumLevel: $databaseRow->nullableInt('minimum_level'),
                vocations: $this->decodeStringList($databaseRow->nullableString('vocations')),
                imageKey: $databaseRow->nullableString('image_key'),
            );
        }

        return new PublicCatalogItemPage(
            context: $context,
            items: $items,
            categories: $categories,
            weaponTypes: $weaponTypes,
            query: $query,
            category: $category,
            weaponType: $weaponType,
            page: $page,
            perPage: self::PER_PAGE,
            total: $total,
        );
    }

    public function item(string $locale, string $slug): ?PublicCatalogItemDetail
    {
        $context = $this->contexts->resolve();
        if ($context === null) {
            return null;
        }

        $row = $this->baseItemQuery($context, $locale)
            ->where(function (Builder $builder) use ($slug): void {
                $builder->where('translation.slug', $slug)
                    ->orWhere('entity.canonical_key', 'item:'.$slug);
            })
            ->first([
                'entity.id as entity_id',
                'entity.canonical_key',
                'translation.slug as translation_slug',
                'translation.display_name as translated_name',
                'translation.summary as translated_summary',
                'item.name',
                'item.description',
                'item.category',
                'item.weapon_type',
                'item.server_id',
                'item.client_id',
                'item.attack',
                'item.defense',
                'item.extra_defense',
                'item.armor',
                'item.range',
                'item.weight',
                'item.minimum_level',
                'item.vocations',
                'item.imbuement_slots',
                'item.element_type',
                'item.element_value',
                'item.stackable',
                'item.pickupable',
                'item.image_key',
            ]);

        if ($row === null) {
            return null;
        }

        $databaseRow = CatalogDatabaseRow::from($row);

        return new PublicCatalogItemDetail(
            slug: $databaseRow->nullableString('translation_slug') ?? $this->slugFromCanonicalKey($databaseRow->string('canonical_key'), 'item'),
            name: $databaseRow->nullableString('translated_name') ?? $databaseRow->string('name'),
            description: $databaseRow->nullableString('translated_summary') ?? $databaseRow->nullableString('description'),
            category: $databaseRow->string('category'),
            weaponType: $databaseRow->nullableString('weapon_type'),
            serverId: $databaseRow->int('server_id'),
            clientId: $databaseRow->nullableInt('client_id'),
            attack: $databaseRow->nullableInt('attack'),
            defense: $databaseRow->nullableInt('defense'),
            extraDefense: $databaseRow->nullableInt('extra_defense'),
            armor: $databaseRow->nullableInt('armor'),
            range: $databaseRow->nullableInt('range'),
            weight: $databaseRow->nullableInt('weight'),
            minimumLevel: $databaseRow->nullableInt('minimum_level'),
            vocations: $this->decodeStringList($databaseRow->nullableString('vocations')),
            imbuementSlots: $databaseRow->nullableInt('imbuement_slots'),
            elementType: $databaseRow->nullableString('element_type'),
            elementValue: $databaseRow->nullableInt('element_value'),
            stackable: $databaseRow->bool('stackable'),
            pickupable: $databaseRow->bool('pickupable'),
            imageKey: $databaseRow->nullableString('image_key'),
            sources: $this->itemSources($context, $locale, $databaseRow->int('entity_id')),
        );
    }

    public function creatures(
        string $locale,
        string $query,
        ?string $bestiaryClass,
        bool $bossOnly,
        int $page,
    ): ?PublicCatalogCreaturePage {
        $context = $this->contexts->resolve();
        if ($context === null) {
            return null;
        }

        $base = $this->baseCreatureQuery($context, $locale);
        $bestiaryClasses = $this->distinctStrings(clone $base, 'creature.bestiary_class');
        $filtered = clone $base;

        if ($query !== '') {
            $like = '%'.$this->escapeLike($query).'%';
            $filtered->where(function (Builder $builder) use ($like): void {
                $builder->where('creature.name', 'like', $like)
                    ->orWhere('translation.display_name', 'like', $like);
            });
        }
        if ($bestiaryClass !== null) {
            $filtered->where('creature.bestiary_class', $bestiaryClass);
        }
        if ($bossOnly) {
            $filtered->where('creature.is_boss', true);
        }

        $total = $filtered->count();
        $rows = $filtered
            ->orderByRaw('COALESCE(translation.display_name, creature.name) ASC')
            ->orderBy('entity.canonical_key')
            ->forPage($page, self::PER_PAGE)
            ->get([
                'entity.canonical_key',
                'translation.slug as translation_slug',
                'translation.display_name as translated_name',
                'translation.summary as translated_summary',
                'creature.name',
                'creature.description',
                'creature.health',
                'creature.experience',
                'creature.bestiary_class',
                'creature.is_boss',
                'creature.look_type',
            ]);

        $creatures = [];
        foreach ($rows as $row) {
            $databaseRow = CatalogDatabaseRow::from($row);
            $creatures[] = new PublicCatalogCreatureCard(
                slug: $databaseRow->nullableString('translation_slug') ?? $this->slugFromCanonicalKey($databaseRow->string('canonical_key'), 'creature'),
                name: $databaseRow->nullableString('translated_name') ?? $databaseRow->string('name'),
                summary: $databaseRow->nullableString('translated_summary') ?? $databaseRow->nullableString('description'),
                health: $databaseRow->int('health'),
                experience: $databaseRow->int('experience'),
                bestiaryClass: $databaseRow->nullableString('bestiary_class'),
                boss: $databaseRow->bool('is_boss'),
                lookType: $databaseRow->nullableInt('look_type'),
            );
        }

        return new PublicCatalogCreaturePage(
            context: $context,
            creatures: $creatures,
            bestiaryClasses: $bestiaryClasses,
            query: $query,
            bestiaryClass: $bestiaryClass,
            bossOnly: $bossOnly,
            page: $page,
            perPage: self::PER_PAGE,
            total: $total,
        );
    }

    public function creature(string $locale, string $slug): ?PublicCatalogCreatureDetail
    {
        $context = $this->contexts->resolve();
        if ($context === null) {
            return null;
        }

        $row = $this->baseCreatureQuery($context, $locale)
            ->where(function (Builder $builder) use ($slug): void {
                $builder->where('translation.slug', $slug)
                    ->orWhere('entity.canonical_key', 'creature:'.$slug);
            })
            ->first([
                'entity.id as entity_id',
                'entity.canonical_key',
                'translation.slug as translation_slug',
                'translation.display_name as translated_name',
                'translation.summary as translated_summary',
                'creature.name',
                'creature.description',
                'creature.health',
                'creature.max_health',
                'creature.experience',
                'creature.speed',
                'creature.armor',
                'creature.defense',
                'creature.mitigation',
                'creature.is_boss',
                'creature.is_reward_boss',
                'creature.bestiary_class',
                'creature.bestiary_race',
                'creature.bestiary_occurrence',
                'creature.bestiary_to_kill',
                'creature.charm_points',
                'creature.look_type',
            ]);

        if ($row === null) {
            return null;
        }

        $databaseRow = CatalogDatabaseRow::from($row);

        return new PublicCatalogCreatureDetail(
            slug: $databaseRow->nullableString('translation_slug') ?? $this->slugFromCanonicalKey($databaseRow->string('canonical_key'), 'creature'),
            name: $databaseRow->nullableString('translated_name') ?? $databaseRow->string('name'),
            description: $databaseRow->nullableString('translated_summary') ?? $databaseRow->nullableString('description'),
            health: $databaseRow->int('health'),
            maxHealth: $databaseRow->int('max_health'),
            experience: $databaseRow->int('experience'),
            speed: $databaseRow->int('speed'),
            armor: $databaseRow->int('armor'),
            defense: $databaseRow->int('defense'),
            mitigation: $databaseRow->nullableString('mitigation'),
            boss: $databaseRow->bool('is_boss'),
            rewardBoss: $databaseRow->bool('is_reward_boss'),
            bestiaryClass: $databaseRow->nullableString('bestiary_class'),
            bestiaryRace: $databaseRow->nullableString('bestiary_race'),
            bestiaryOccurrence: $databaseRow->nullableInt('bestiary_occurrence'),
            bestiaryToKill: $databaseRow->nullableInt('bestiary_to_kill'),
            charmPoints: $databaseRow->nullableInt('charm_points'),
            lookType: $databaseRow->nullableInt('look_type'),
            loot: $this->creatureLoot($context, $locale, $databaseRow->int('entity_id')),
        );
    }

    private function baseItemQuery(PublicCatalogContext $context, string $locale): Builder
    {
        return DB::table('game_catalog_profile_entities as visibility')
            ->join('game_catalog_entity_snapshots as entity_snapshot', 'entity_snapshot.id', '=', 'visibility.entity_snapshot_id')
            ->join('game_catalog_entities as entity', 'entity.id', '=', 'entity_snapshot.entity_id')
            ->join('game_catalog_item_snapshots as item', 'item.entity_snapshot_id', '=', 'entity_snapshot.id')
            ->leftJoin('game_catalog_entity_translations as translation', function (JoinClause $join) use ($locale): void {
                $join->on('translation.entity_id', '=', 'entity.id')
                    ->where('translation.locale', '=', $locale)
                    ->where('translation.translation_status', '=', 'approved');
            })
            ->where('visibility.profile_id', $context->profileId)
            ->where('visibility.visible', true)
            ->where('entity_snapshot.snapshot_id', $context->snapshotId)
            ->where('entity.entity_type', 'item');
    }

    private function baseCreatureQuery(PublicCatalogContext $context, string $locale): Builder
    {
        return DB::table('game_catalog_profile_entities as visibility')
            ->join('game_catalog_entity_snapshots as entity_snapshot', 'entity_snapshot.id', '=', 'visibility.entity_snapshot_id')
            ->join('game_catalog_entities as entity', 'entity.id', '=', 'entity_snapshot.entity_id')
            ->join('game_catalog_creature_snapshots as creature', 'creature.entity_snapshot_id', '=', 'entity_snapshot.id')
            ->leftJoin('game_catalog_entity_translations as translation', function (JoinClause $join) use ($locale): void {
                $join->on('translation.entity_id', '=', 'entity.id')
                    ->where('translation.locale', '=', $locale)
                    ->where('translation.translation_status', '=', 'approved');
            })
            ->where('visibility.profile_id', $context->profileId)
            ->where('visibility.visible', true)
            ->where('entity_snapshot.snapshot_id', $context->snapshotId)
            ->where('entity.entity_type', 'creature');
    }

    /** @return list<PublicCatalogLootEntry> */
    private function itemSources(PublicCatalogContext $context, string $locale, int $itemEntityId): array
    {
        $rows = DB::table('game_catalog_profile_relations as relation_visibility')
            ->join('game_catalog_relation_snapshots as relation', 'relation.id', '=', 'relation_visibility.relation_snapshot_id')
            ->join('game_catalog_loot_snapshots as loot', 'loot.relation_snapshot_id', '=', 'relation.id')
            ->join('game_catalog_entities as source_entity', 'source_entity.id', '=', 'relation.source_entity_id')
            ->join('game_catalog_entity_snapshots as source_snapshot', function (JoinClause $join): void {
                $join->on('source_snapshot.entity_id', '=', 'source_entity.id')
                    ->on('source_snapshot.snapshot_id', '=', 'relation.snapshot_id');
            })
            ->join('game_catalog_profile_entities as source_visibility', function (JoinClause $join) use ($context): void {
                $join->on('source_visibility.entity_snapshot_id', '=', 'source_snapshot.id')
                    ->where('source_visibility.profile_id', '=', $context->profileId)
                    ->where('source_visibility.visible', '=', true);
            })
            ->join('game_catalog_creature_snapshots as creature', 'creature.entity_snapshot_id', '=', 'source_snapshot.id')
            ->leftJoin('game_catalog_entity_translations as translation', function (JoinClause $join) use ($locale): void {
                $join->on('translation.entity_id', '=', 'source_entity.id')
                    ->where('translation.locale', '=', $locale)
                    ->where('translation.translation_status', '=', 'approved');
            })
            ->where('relation_visibility.profile_id', $context->profileId)
            ->where('relation_visibility.visible', true)
            ->where('relation.snapshot_id', $context->snapshotId)
            ->where('relation.relation_type', 'creature_loot')
            ->where('relation.target_entity_id', $itemEntityId)
            ->orderByRaw('COALESCE(translation.display_name, creature.name) ASC')
            ->get([
                'source_entity.canonical_key',
                'translation.slug as translation_slug',
                'translation.display_name as translated_name',
                'creature.name',
                'loot.chance_model',
                'loot.chance_numerator',
                'loot.chance_denominator',
                'loot.chance_threshold',
                'loot.roll_maximum',
                'loot.minimum_count',
                'loot.maximum_count',
            ]);

        return $this->lootEntries($rows->all(), 'creature');
    }

    /** @return list<PublicCatalogLootEntry> */
    private function creatureLoot(PublicCatalogContext $context, string $locale, int $creatureEntityId): array
    {
        $rows = DB::table('game_catalog_profile_relations as relation_visibility')
            ->join('game_catalog_relation_snapshots as relation', 'relation.id', '=', 'relation_visibility.relation_snapshot_id')
            ->join('game_catalog_loot_snapshots as loot', 'loot.relation_snapshot_id', '=', 'relation.id')
            ->join('game_catalog_entities as target_entity', 'target_entity.id', '=', 'relation.target_entity_id')
            ->join('game_catalog_entity_snapshots as target_snapshot', function (JoinClause $join): void {
                $join->on('target_snapshot.entity_id', '=', 'target_entity.id')
                    ->on('target_snapshot.snapshot_id', '=', 'relation.snapshot_id');
            })
            ->join('game_catalog_profile_entities as target_visibility', function (JoinClause $join) use ($context): void {
                $join->on('target_visibility.entity_snapshot_id', '=', 'target_snapshot.id')
                    ->where('target_visibility.profile_id', '=', $context->profileId)
                    ->where('target_visibility.visible', '=', true);
            })
            ->join('game_catalog_item_snapshots as item', 'item.entity_snapshot_id', '=', 'target_snapshot.id')
            ->leftJoin('game_catalog_entity_translations as translation', function (JoinClause $join) use ($locale): void {
                $join->on('translation.entity_id', '=', 'target_entity.id')
                    ->where('translation.locale', '=', $locale)
                    ->where('translation.translation_status', '=', 'approved');
            })
            ->where('relation_visibility.profile_id', $context->profileId)
            ->where('relation_visibility.visible', true)
            ->where('relation.snapshot_id', $context->snapshotId)
            ->where('relation.relation_type', 'creature_loot')
            ->where('relation.source_entity_id', $creatureEntityId)
            ->orderByRaw('COALESCE(translation.display_name, item.name) ASC')
            ->get([
                'target_entity.canonical_key',
                'translation.slug as translation_slug',
                'translation.display_name as translated_name',
                'item.name',
                'loot.chance_model',
                'loot.chance_numerator',
                'loot.chance_denominator',
                'loot.chance_threshold',
                'loot.roll_maximum',
                'loot.minimum_count',
                'loot.maximum_count',
            ]);

        return $this->lootEntries($rows->all(), 'item');
    }

    /**
     * @param  array<int, object>  $rows
     * @return list<PublicCatalogLootEntry>
     */
    private function lootEntries(array $rows, string $entityType): array
    {
        $entries = [];
        foreach ($rows as $row) {
            $databaseRow = CatalogDatabaseRow::from($row);
            $entries[] = new PublicCatalogLootEntry(
                entityType: $entityType,
                slug: $databaseRow->nullableString('translation_slug') ?? $this->slugFromCanonicalKey($databaseRow->string('canonical_key'), $entityType),
                name: $databaseRow->nullableString('translated_name') ?? $databaseRow->string('name'),
                chanceModel: $databaseRow->string('chance_model'),
                chanceNumerator: $databaseRow->nullableInt('chance_numerator'),
                chanceDenominator: $databaseRow->nullableInt('chance_denominator'),
                chanceThreshold: $databaseRow->nullableInt('chance_threshold'),
                rollMaximum: $databaseRow->nullableInt('roll_maximum'),
                minimumCount: $databaseRow->int('minimum_count'),
                maximumCount: $databaseRow->int('maximum_count'),
            );
        }

        return $entries;
    }

    /** @return list<string> */
    private function distinctStrings(Builder $query, string $column): array
    {
        $values = $query
            ->whereNotNull($column)
            ->distinct()
            ->orderBy($column)
            ->pluck($column)
            ->all();

        $strings = [];
        foreach ($values as $value) {
            if (is_string($value) && $value !== '') {
                $strings[] = $value;
            }
        }

        return $strings;
    }

    /** @return list<string> */
    private function decodeStringList(?string $json): array
    {
        if ($json === null) {
            return [];
        }

        try {
            $decoded = json_decode($json, true, 32, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new RuntimeException('Persisted Game Catalog vocation data is invalid.', previous: $exception);
        }

        if (! is_array($decoded) || ! array_is_list($decoded)) {
            throw new RuntimeException('Persisted Game Catalog vocation data is not a list.');
        }

        $values = [];
        foreach ($decoded as $value) {
            if (! is_string($value)) {
                throw new RuntimeException('Persisted Game Catalog vocation data contains a non-string value.');
            }
            $values[] = $value;
        }

        return $values;
    }

    private function slugFromCanonicalKey(string $canonicalKey, string $entityType): string
    {
        $prefix = $entityType.':';
        if (! str_starts_with($canonicalKey, $prefix)) {
            throw new RuntimeException('Persisted Game Catalog canonical key has an unexpected namespace.');
        }

        $slug = substr($canonicalKey, strlen($prefix));
        if ($slug === '' || preg_match('/^[a-z0-9]+(?:-[a-z0-9]+)*$/D', $slug) !== 1) {
            throw new RuntimeException('Persisted Game Catalog canonical key cannot be exposed as a public slug.');
        }

        return $slug;
    }

    private function escapeLike(string $value): string
    {
        return str_replace(['\\', '%', '_'], ['\\\\', '\\%', '\\_'], $value);
    }
}
