<?php

namespace App\Wiki\Application;

use App\Identity\Models\Identity;
use App\Wiki\Domain\WikiCategoryTranslationInput;
use App\Wiki\Infrastructure\Models\WikiCategory;
use DomainException;

final readonly class WikiAdminCategoryWriter
{
    public function __construct(private WikiCategoryService $categories) {}

    /** @param list<WikiCategoryTranslationInput> $translations */
    public function create(
        Identity $actor,
        string $key,
        array $translations,
        ?WikiCategory $parent,
        int $sortOrder,
        bool $visible,
    ): WikiCategory {
        return $this->categories->create($actor, $key, $translations, $parent, $sortOrder, $visible);
    }

    /** @param list<WikiCategoryTranslationInput> $translations */
    public function update(
        Identity $actor,
        WikiCategory $category,
        int $expectedVersion,
        string $key,
        array $translations,
        ?WikiCategory $parent,
        int $sortOrder,
        bool $visible,
    ): WikiCategory {
        $this->assertAcyclicParent($category, $parent);

        return $this->categories->update(
            $actor,
            $category,
            $expectedVersion,
            $key,
            $translations,
            $parent,
            $sortOrder,
            $visible,
        );
    }

    private function assertAcyclicParent(WikiCategory $category, ?WikiCategory $parent): void
    {
        $cursor = $parent;
        $visited = [];

        while ($cursor !== null) {
            if ($cursor->id === $category->id) {
                throw new DomainException('A Wiki category parent cannot be the category itself or one of its descendants.');
            }

            if (isset($visited[$cursor->id])) {
                throw new DomainException('The existing Wiki category tree contains a cycle.');
            }

            $visited[$cursor->id] = true;

            if (count($visited) > 100) {
                throw new DomainException('The Wiki category hierarchy exceeds the supported depth.');
            }

            $cursor = $cursor->parent_id === null
                ? null
                : WikiCategory::query()->find($cursor->parent_id);
        }
    }
}
