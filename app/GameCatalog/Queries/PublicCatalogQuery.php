<?php

namespace App\GameCatalog\Queries;

use Illuminate\Contracts\Pagination\LengthAwarePaginator;
use Illuminate\Database\Query\Builder;
use Illuminate\Support\Facades\DB;

final class PublicCatalogQuery
{
    private bool $contextLoaded = false;

    private ?object $context = null;

    public function context(): ?object
    {
        if ($this->contextLoaded) {
            return $this->context;
        }
        $this->contextLoaded = true;
        $profileKey = config('game-catalog.public_profile');
        if (! is_string($profileKey) || $profileKey === '') {
            return null;
        }

        $this->context = DB::table('game_catalog_profiles as profiles')
            ->join('game_catalog_snapshots as snapshots', 'snapshots.id', '=', 'profiles.active_snapshot_id')
            ->join('game_catalog_releases as target_release', 'target_release.id', '=', 'profiles.target_release_id')
            ->join('game_catalog_releases as content_release', 'content_release.id', '=', 'snapshots.content_target_release_id')
            ->where('profiles.key', $profileKey)
            ->where('profiles.public_enabled', true)
            ->where('snapshots.status', 'validated')
            ->first([
                'profiles.id as profile_id',
                'profiles.key as profile_key',
                'profiles.name as profile_name',
                'profiles.active_snapshot_id as snapshot_id',
                'target_release.key as target_release',
                'target_release.display_label as target_release_label',
                'content_release.key as snapshot_content_release',
                'snapshots.generated_at',
                'snapshots.content_sha256',
            ]);

        return $this->context;
    }

    /**
     * @param  array{category?: string|null, weapon_type?: string|null, q?: string|null, per_page?: int}  $filters
     * @return LengthAwarePaginator<object>
     */
    public function items(array $filters): LengthAwarePaginator
    {
        $query = $this->visibleEntityQuery('item')
            ->join('game_catalog_item_snapshots as items', 'items.entity_snapshot_id', '=', 'versions.id')
            ->select([
                'entities.id as entity_id',
                'entities.canonical_key',
                'versions.availability',
                'items.name',
                'items.category',
                'items.weapon_type',
                'items.attack',
                'items.defense',
                'items.extra_defense',
                'items.armor',
                'items.range',
                'items.weight',
                'items.minimum_level',
                'items.image_key',
            ]);

        if (($filters['category'] ?? null) !== null) {
            $query->where('items.category', $filters['category']);
        }
        if (($filters['weapon_type'] ?? null) !== null) {
            $query->where('items.weapon_type', $filters['weapon_type']);
        }
        if (($filters['q'] ?? null) !== null) {
            $query->where('items.name', 'like', '%'.$this->escapeLike($filters['q']).'%');
        }

        return $query
            ->orderBy('items.name')
            ->orderBy('entities.canonical_key')
            ->paginate($filters['per_page'] ?? 24)
            ->withQueryString();
    }

    /** @return list<object> */
    public function itemCategories(): array
    {
        return $this->visibleEntityQuery('item')
            ->join('game_catalog_item_snapshots as items', 'items.entity_snapshot_id', '=', 'versions.id')
            ->selectRaw('items.category, items.weapon_type, COUNT(*) as item_count')
            ->groupBy('items.category', 'items.weapon_type')
            ->orderBy('items.category')
            ->orderBy('items.weapon_type')
            ->get()
            ->all();
    }

    public function item(string $slug): ?object
    {
        return $this->visibleEntityQuery('item')
            ->join('game_catalog_item_snapshots as items', 'items.entity_snapshot_id', '=', 'versions.id')
            ->where('entities.canonical_key', 'item:'.$slug)
            ->first([
                'entities.id as entity_id',
                'entities.canonical_key',
                'versions.availability',
                'versions.completeness',
                'items.*',
            ]);
    }

    /** @return list<object> */
    public function itemLootSources(int $entityId): array
    {
        $context = $this->context();
        if ($context === null) {
            return [];
        }

        return DB::table('game_catalog_profile_relations as visibility')
            ->join('game_catalog_relation_snapshots as relations', 'relations.id', '=', 'visibility.relation_snapshot_id')
            ->join('game_catalog_loot_snapshots as loot', 'loot.relation_snapshot_id', '=', 'relations.id')
            ->join('game_catalog_entities as creatures', 'creatures.id', '=', 'relations.source_entity_id')
            ->join('game_catalog_entity_snapshots as creature_versions', function ($join) use ($context): void {
                $join->on('creature_versions.entity_id', '=', 'creatures.id')
                    ->where('creature_versions.snapshot_id', '=', $context->snapshot_id);
            })
            ->join('game_catalog_creature_snapshots as creature_data', 'creature_data.entity_snapshot_id', '=', 'creature_versions.id')
            ->where('visibility.profile_id', $context->profile_id)
            ->where('visibility.visible', true)
            ->where('relations.snapshot_id', $context->snapshot_id)
            ->where('relations.target_entity_id', $entityId)
            ->orderBy('creature_data.name')
            ->get([
                'creatures.canonical_key',
                'creature_data.name',
                'loot.chance_numerator',
                'loot.chance_denominator',
                'loot.minimum_count',
                'loot.maximum_count',
                'loot.container_path',
                'loot.condition_data',
            ])
            ->all();
    }

    /**
     * @param  array{q?: string|null, boss?: bool|null, bestiary_class?: string|null, per_page?: int}  $filters
     * @return LengthAwarePaginator<object>
     */
    public function creatures(array $filters): LengthAwarePaginator
    {
        $query = $this->visibleEntityQuery('creature')
            ->join('game_catalog_creature_snapshots as creatures', 'creatures.entity_snapshot_id', '=', 'versions.id')
            ->select([
                'entities.id as entity_id',
                'entities.canonical_key',
                'versions.availability',
                'creatures.name',
                'creatures.health',
                'creatures.experience',
                'creatures.speed',
                'creatures.armor',
                'creatures.defense',
                'creatures.is_boss',
                'creatures.bestiary_class',
                'creatures.bestiary_to_kill',
                'creatures.charm_points',
            ]);

        if (($filters['q'] ?? null) !== null) {
            $query->where('creatures.name', 'like', '%'.$this->escapeLike($filters['q']).'%');
        }
        if (($filters['boss'] ?? null) !== null) {
            $query->where('creatures.is_boss', $filters['boss']);
        }
        if (($filters['bestiary_class'] ?? null) !== null) {
            $query->where('creatures.bestiary_class', $filters['bestiary_class']);
        }

        return $query
            ->orderBy('creatures.name')
            ->orderBy('entities.canonical_key')
            ->paginate($filters['per_page'] ?? 24)
            ->withQueryString();
    }

    /** @return list<string> */
    public function bestiaryClasses(): array
    {
        return $this->visibleEntityQuery('creature')
            ->join('game_catalog_creature_snapshots as creatures', 'creatures.entity_snapshot_id', '=', 'versions.id')
            ->whereNotNull('creatures.bestiary_class')
            ->distinct()
            ->orderBy('creatures.bestiary_class')
            ->pluck('creatures.bestiary_class')
            ->map(static fn (mixed $value): string => (string) $value)
            ->all();
    }

    public function creature(string $slug): ?object
    {
        return $this->visibleEntityQuery('creature')
            ->join('game_catalog_creature_snapshots as creatures', 'creatures.entity_snapshot_id', '=', 'versions.id')
            ->where('entities.canonical_key', 'creature:'.$slug)
            ->first([
                'entities.id as entity_id',
                'entities.canonical_key',
                'versions.availability',
                'versions.completeness',
                'creatures.*',
            ]);
    }

    /** @return list<object> */
    public function creatureLoot(int $entityId): array
    {
        $context = $this->context();
        if ($context === null) {
            return [];
        }

        return DB::table('game_catalog_profile_relations as visibility')
            ->join('game_catalog_relation_snapshots as relations', 'relations.id', '=', 'visibility.relation_snapshot_id')
            ->join('game_catalog_loot_snapshots as loot', 'loot.relation_snapshot_id', '=', 'relations.id')
            ->join('game_catalog_entities as items', 'items.id', '=', 'relations.target_entity_id')
            ->join('game_catalog_entity_snapshots as item_versions', function ($join) use ($context): void {
                $join->on('item_versions.entity_id', '=', 'items.id')
                    ->where('item_versions.snapshot_id', '=', $context->snapshot_id);
            })
            ->join('game_catalog_item_snapshots as item_data', 'item_data.entity_snapshot_id', '=', 'item_versions.id')
            ->where('visibility.profile_id', $context->profile_id)
            ->where('visibility.visible', true)
            ->where('relations.snapshot_id', $context->snapshot_id)
            ->where('relations.source_entity_id', $entityId)
            ->orderBy('item_data.name')
            ->get([
                'items.canonical_key',
                'item_data.name',
                'item_data.category',
                'item_data.image_key',
                'loot.chance_numerator',
                'loot.chance_denominator',
                'loot.minimum_count',
                'loot.maximum_count',
                'loot.container_path',
                'loot.condition_data',
            ])
            ->all();
    }

    private function visibleEntityQuery(string $type): Builder
    {
        $context = $this->context();
        if ($context === null) {
            return DB::table('game_catalog_entity_snapshots as versions')->whereRaw('1 = 0');
        }

        return DB::table('game_catalog_profile_entities as visibility')
            ->join('game_catalog_entity_snapshots as versions', 'versions.id', '=', 'visibility.entity_snapshot_id')
            ->join('game_catalog_entities as entities', 'entities.id', '=', 'versions.entity_id')
            ->where('visibility.profile_id', $context->profile_id)
            ->where('visibility.visible', true)
            ->where('versions.snapshot_id', $context->snapshot_id)
            ->where('entities.entity_type', $type);
    }

    private function escapeLike(string $value): string
    {
        return addcslashes($value, '%_\\');
    }
}
