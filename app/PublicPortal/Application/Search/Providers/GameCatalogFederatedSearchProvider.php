<?php

namespace App\PublicPortal\Application\Search\Providers;

use App\GameCatalog\Queries\Public\DatabasePublicCatalogQuery;
use App\GameCatalog\Queries\Public\PublicCatalogCreatureCard;
use App\GameCatalog\Queries\Public\PublicCatalogItemCard;
use App\GameCatalog\Queries\Public\PublicCatalogSummary;
use App\PublicPortal\Application\Search\FederatedSearchProvider;
use App\PublicPortal\Application\Search\FederatedSearchQuery;
use App\PublicPortal\Application\Search\SearchHit;
use App\PublicPortal\Application\Search\SearchProviderResult;
use RuntimeException;

final readonly class GameCatalogFederatedSearchProvider implements FederatedSearchProvider
{
    public function __construct(private DatabasePublicCatalogQuery $catalog) {}

    public function id(): string
    {
        return 'game_catalog';
    }

    public function search(FederatedSearchQuery $query): SearchProviderResult
    {
        $before = $this->catalog->summary();
        if ($before === null) {
            return SearchProviderResult::unavailable($this->id());
        }

        $itemEnabled = $query->allowsType('catalog_item');
        $creatureEnabled = $query->allowsType('catalog_creature');
        [$itemLimit, $creatureLimit] = $this->budgets($query->limit, $itemEnabled, $creatureEnabled);

        $items = [];
        if ($itemLimit > 0) {
            $rows = $this->catalog->searchItems(
                $query->locale,
                $query->query,
                ($query->page - 1) * $itemLimit,
                $itemLimit,
            );
            if ($rows === null) {
                return SearchProviderResult::unavailable($this->id());
            }
            foreach ($rows as $row) {
                $items[] = $this->itemHit($row, $query, $before);
            }
        }

        $creatures = [];
        if ($creatureLimit > 0) {
            $rows = $this->catalog->searchCreatures(
                $query->locale,
                $query->query,
                ($query->page - 1) * $creatureLimit,
                $creatureLimit,
            );
            if ($rows === null) {
                return SearchProviderResult::unavailable($this->id());
            }
            foreach ($rows as $row) {
                $creatures[] = $this->creatureHit($row, $query, $before);
            }
        }

        $after = $this->catalog->summary();
        if (
            $after === null
            || $after->context->snapshotId !== $before->context->snapshotId
            || ! hash_equals($after->context->snapshotSha256, $before->context->snapshotSha256)
        ) {
            throw new RuntimeException('Game Catalog active snapshot changed during federated search.');
        }

        return new SearchProviderResult(
            provider: $this->id(),
            available: true,
            hits: ProviderSearchSupport::interleave($query->limit, $items, $creatures),
        );
    }

    /** @return array{0:int,1:int} */
    private function budgets(int $limit, bool $items, bool $creatures): array
    {
        if ($items && $creatures) {
            return [(int) ceil($limit / 2), intdiv($limit, 2)];
        }

        return [$items ? $limit : 0, $creatures ? $limit : 0];
    }

    private function itemHit(
        PublicCatalogItemCard $item,
        FederatedSearchQuery $query,
        PublicCatalogSummary $summary,
    ): SearchHit {
        return new SearchHit(
            sourceModule: 'GameCatalog',
            sourceType: 'catalog_item',
            stablePublicId: 'catalog-item-'.$item->slug,
            sourceRevision: $summary->context->snapshotSha256,
            locale: $query->locale,
            title: $item->name,
            snippetPlaintext: ProviderSearchSupport::snippet($item->summary),
            canonicalUrl: route('game-catalog.items.show', ['locale' => $query->locale, 'slug' => $item->slug]),
            providerRank: 0,
            applicability: $summary->context->targetRelease,
            freshness: $summary->context->generatedAt,
            badges: [$item->category],
        );
    }

    private function creatureHit(
        PublicCatalogCreatureCard $creature,
        FederatedSearchQuery $query,
        PublicCatalogSummary $summary,
    ): SearchHit {
        $badges = [];
        if ($creature->bestiaryClass !== null) {
            $badges[] = $creature->bestiaryClass;
        }
        if ($creature->boss) {
            $badges[] = 'boss';
        }

        return new SearchHit(
            sourceModule: 'GameCatalog',
            sourceType: 'catalog_creature',
            stablePublicId: 'catalog-creature-'.$creature->slug,
            sourceRevision: $summary->context->snapshotSha256,
            locale: $query->locale,
            title: $creature->name,
            snippetPlaintext: ProviderSearchSupport::snippet($creature->summary),
            canonicalUrl: route('game-catalog.creatures.show', ['locale' => $query->locale, 'slug' => $creature->slug]),
            providerRank: 0,
            applicability: $summary->context->targetRelease,
            freshness: $summary->context->generatedAt,
            badges: $badges,
        );
    }
}
