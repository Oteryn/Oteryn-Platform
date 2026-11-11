<?php

namespace App\Wiki\Application;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Wiki\Domain\Exceptions\StaleWikiEdit;
use App\Wiki\Domain\WikiCategoryTranslationInput;
use App\Wiki\Domain\WikiContentRules;
use App\Wiki\Infrastructure\Audit\WikiAuditAction;
use App\Wiki\Infrastructure\Models\WikiCategory;
use App\Wiki\Infrastructure\Models\WikiCategoryTranslation;
use DomainException;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;

final readonly class WikiCategoryService
{
    public function __construct(
        private WikiAuthorization $authorization,
        private AdminAuditRecorder $audit,
    ) {}

    /**
     * @param  list<WikiCategoryTranslationInput>  $translations
     */
    public function create(
        Identity $actor,
        string $key,
        array $translations,
        ?WikiCategory $parent = null,
        int $sortOrder = 0,
        bool $visible = true,
    ): WikiCategory {
        $this->authorization->assertCanManageCategories($actor);
        WikiContentRules::assertCategoryKey($key);
        $this->assertTranslationSet($translations);

        return DB::transaction(function () use (
            $actor,
            $key,
            $translations,
            $parent,
            $sortOrder,
            $visible,
        ): WikiCategory {
            if ($parent !== null && ! WikiCategory::query()->whereKey($parent->id)->exists()) {
                throw new InvalidArgumentException('The parent Wiki category does not exist.');
            }

            $category = WikiCategory::query()->create([
                'parent_id' => $parent?->id,
                'key' => $key,
                'sort_order' => $sortOrder,
                'visible' => $visible,
                'lock_version' => 1,
            ]);

            foreach ($translations as $input) {
                $this->assertSlugAvailable($input->locale, $input->slug, null);
                $this->saveTranslation($category->id, $input);
            }

            $this->audit->record(
                $actor->id,
                WikiAuditAction::CATEGORY_CREATED,
                'wiki_category',
                (string) $category->id,
                [
                    'key' => $category->key,
                    'version' => $category->lock_version,
                    'locales' => $this->localeList($translations),
                ],
            );

            return $category;
        }, 3);
    }

    /**
     * @param  list<WikiCategoryTranslationInput>  $translations
     */
    public function update(
        Identity $actor,
        WikiCategory $category,
        int $expectedVersion,
        string $key,
        array $translations,
        ?WikiCategory $parent = null,
        int $sortOrder = 0,
        bool $visible = true,
    ): WikiCategory {
        $this->authorization->assertCanManageCategories($actor);
        WikiContentRules::assertCategoryKey($key);
        $this->assertTranslationSet($translations);

        return DB::transaction(function () use (
            $actor,
            $category,
            $expectedVersion,
            $key,
            $translations,
            $parent,
            $sortOrder,
            $visible,
        ): WikiCategory {
            $current = WikiCategory::query()->lockForUpdate()->findOrFail($category->id);

            if ($current->lock_version !== $expectedVersion) {
                throw new StaleWikiEdit('The Wiki category changed after this edit began.');
            }

            if ($parent !== null) {
                if ($parent->id === $current->id) {
                    throw new DomainException('A Wiki category cannot be its own parent.');
                }

                if (! WikiCategory::query()->whereKey($parent->id)->exists()) {
                    throw new InvalidArgumentException('The parent Wiki category does not exist.');
                }
            }

            $current->forceFill([
                'parent_id' => $parent?->id,
                'key' => $key,
                'sort_order' => $sortOrder,
                'visible' => $visible,
                'lock_version' => $current->lock_version + 1,
            ])->save();

            foreach ($translations as $input) {
                $this->assertSlugAvailable($input->locale, $input->slug, $current->id);
                $this->saveTranslation($current->id, $input);
            }

            $this->audit->record(
                $actor->id,
                WikiAuditAction::CATEGORY_UPDATED,
                'wiki_category',
                (string) $current->id,
                [
                    'key' => $current->key,
                    'version' => $current->lock_version,
                    'locales' => $this->localeList($translations),
                ],
            );

            return $current;
        }, 3);
    }

    /**
     * @param  list<WikiCategoryTranslationInput>  $translations
     */
    private function assertTranslationSet(array $translations): void
    {
        if ($translations === []) {
            throw new InvalidArgumentException('At least one Wiki category translation is required.');
        }

        $locales = array_map(
            static fn (WikiCategoryTranslationInput $input): string => $input->locale,
            $translations,
        );

        if (count($locales) !== count(array_unique($locales))) {
            throw new InvalidArgumentException('A Wiki category translation locale may appear only once per write.');
        }
    }

    private function assertSlugAvailable(string $locale, string $slug, ?int $exceptCategoryId): void
    {
        $query = WikiCategoryTranslation::query()
            ->where('locale', $locale)
            ->where('slug', $slug);

        if ($exceptCategoryId !== null) {
            $query->where('category_id', '!=', $exceptCategoryId);
        }

        if ($query->exists()) {
            throw new DomainException('The localized Wiki category slug is already in use.');
        }
    }

    private function saveTranslation(int $categoryId, WikiCategoryTranslationInput $input): WikiCategoryTranslation
    {
        $translation = WikiCategoryTranslation::query()->firstOrNew([
            'category_id' => $categoryId,
            'locale' => $input->locale,
        ]);

        $translation->fill([
            'name' => $input->name,
            'slug' => $input->slug,
            'description' => $input->description,
        ])->save();

        return $translation;
    }

    /**
     * @param  list<WikiCategoryTranslationInput>  $translations
     */
    private function localeList(array $translations): string
    {
        $locales = array_map(
            static fn (WikiCategoryTranslationInput $input): string => $input->locale,
            $translations,
        );
        sort($locales, SORT_STRING);

        return implode(',', $locales);
    }
}
