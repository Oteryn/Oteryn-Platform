<?php

namespace App\Wiki\Content;

use App\Admin\AdminPermission;
use App\Identity\Models\Identity;
use App\Wiki\Application\WikiAdminArticleWriter;
use App\Wiki\Application\WikiAdminCategoryWriter;
use App\Wiki\Application\WikiArticleService;
use App\Wiki\Application\WikiAuthorization;
use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiArticleTranslation;
use App\Wiki\Infrastructure\Models\WikiCategory;
use App\Wiki\Infrastructure\Models\WikiCategoryTranslation;
use Illuminate\Auth\Access\AuthorizationException;
use Illuminate\Support\Facades\DB;
use LogicException;

final readonly class WikiLaunchContentInstaller
{
    public function __construct(
        private WikiLaunchContentCatalog $catalog,
        private WikiAuthorization $authorization,
        private WikiAdminCategoryWriter $categories,
        private WikiAdminArticleWriter $articles,
        private WikiArticleService $lifecycle,
    ) {}

    public function install(Identity $actor): WikiLaunchContentInstallResult
    {
        $this->assertEligiblePublisher($actor);

        return DB::transaction(function () use ($actor): WikiLaunchContentInstallResult {
            $this->lockInstallationBoundary();

            $categoryModels = [];
            $categoriesToCreate = [];

            foreach ($this->catalog->categories() as $definition) {
                $existing = WikiCategory::query()
                    ->where('key', $definition->key)
                    ->lockForUpdate()
                    ->first();

                if ($existing instanceof WikiCategory) {
                    $this->assertCategoryMatches($existing, $definition);
                    $categoryModels[$definition->key] = $existing;
                } else {
                    $this->assertCategorySlugsAvailable($definition);
                    $categoriesToCreate[] = $definition;
                }
            }

            $articlesToCreate = [];

            foreach ($this->catalog->articles() as $definition) {
                $existing = $this->findArticleByLaunchSlugs($definition);

                if ($existing instanceof WikiArticle) {
                    $this->assertArticleMatches($existing, $definition);
                } else {
                    $articlesToCreate[] = $definition;
                }
            }

            foreach ($categoriesToCreate as $definition) {
                $categoryModels[$definition->key] = $this->categories->create(
                    $actor,
                    $definition->key,
                    $definition->translations,
                    null,
                    $definition->sortOrder,
                    true,
                );
            }

            foreach ($articlesToCreate as $definition) {
                $categoryIds = array_map(
                    static function (string $key) use ($categoryModels): int {
                        $category = $categoryModels[$key] ?? null;

                        if (! $category instanceof WikiCategory) {
                            throw new LogicException("Wiki launch category {$key} is unavailable.");
                        }

                        return $category->id;
                    },
                    $definition->categoryKeys,
                );

                $article = $this->articles->create(
                    $actor,
                    $definition->contentType,
                    array_map(
                        static fn (WikiLaunchTranslation $translation) => $translation->toInput(),
                        $definition->translations,
                    ),
                    'Install reviewed Wiki launch content '.WikiLaunchContentCatalog::VERSION,
                    $definition->featured,
                    $definition->sortOrder,
                    $categoryIds,
                );
                $article = $this->lifecycle->submitForReview($actor, $article, $article->lock_version);
                $this->lifecycle->publish($actor, $article, $article->lock_version);
            }

            return new WikiLaunchContentInstallResult(
                count($categoriesToCreate),
                count($articlesToCreate),
            );
        }, 3);
    }

    private function assertEligiblePublisher(Identity $actor): void
    {
        if ($actor->disabled_at !== null || ! $actor->hasConfirmedMfa()) {
            throw new AuthorizationException(
                'Wiki launch content requires an enabled MFA-confirmed publisher Identity.',
            );
        }

        $this->authorization->assertCanAccess($actor);
        $this->authorization->assertCanManageCategories($actor);
        $this->authorization->assertCanManageArticles($actor);
        $this->authorization->assertCanPublish($actor);
    }

    private function lockInstallationBoundary(): void
    {
        $permissionId = DB::table('admin_permissions')
            ->where('key', AdminPermission::WIKI_ACCESS)
            ->lockForUpdate()
            ->value('id');

        if (! is_int($permissionId)) {
            throw new LogicException('The Wiki access permission is unavailable.');
        }
    }

    private function assertCategorySlugsAvailable(WikiLaunchCategory $definition): void
    {
        foreach ($definition->translations as $translation) {
            if (WikiCategoryTranslation::query()
                ->where('locale', $translation->locale)
                ->where('slug', $translation->slug)
                ->exists()) {
                throw new LogicException(
                    "Wiki launch category slug {$translation->locale}/{$translation->slug} is already in use.",
                );
            }
        }
    }

    private function assertCategoryMatches(
        WikiCategory $category,
        WikiLaunchCategory $definition,
    ): void {
        if (
            $category->parent_id !== null
            || ! $category->visible
            || $category->sort_order !== $definition->sortOrder
        ) {
            throw new LogicException("Wiki launch category {$definition->key} conflicts with reviewed content.");
        }

        $translations = WikiCategoryTranslation::query()
            ->where('category_id', $category->id)
            ->get()
            ->keyBy('locale');

        if ($translations->count() !== count($definition->translations)) {
            throw new LogicException("Wiki launch category {$definition->key} has an unexpected translation set.");
        }

        foreach ($definition->translations as $expected) {
            $translation = $translations->get($expected->locale);

            if (
                ! $translation instanceof WikiCategoryTranslation
                || $translation->name !== $expected->name
                || $translation->slug !== $expected->slug
                || $translation->description !== $expected->description
            ) {
                throw new LogicException(
                    "Wiki launch category {$definition->key} conflicts with reviewed content.",
                );
            }
        }
    }

    private function findArticleByLaunchSlugs(WikiLaunchArticle $definition): ?WikiArticle
    {
        $english = $definition->translation('en');
        $polish = $definition->translation('pl');

        $articleIds = WikiArticleTranslation::query()
            ->where(function ($query) use ($english, $polish): void {
                $query
                    ->where(function ($query) use ($english): void {
                        $query
                            ->where('locale', $english->locale)
                            ->where('slug', $english->slug);
                    })
                    ->orWhere(function ($query) use ($polish): void {
                        $query
                            ->where('locale', $polish->locale)
                            ->where('slug', $polish->slug);
                    });
            })
            ->pluck('article_id')
            ->unique()
            ->values();

        if ($articleIds->isEmpty()) {
            return null;
        }

        if ($articleIds->count() !== 1) {
            throw new LogicException(
                "Wiki launch article {$english->slug} has conflicting localized slug owners.",
            );
        }

        $articleId = $articleIds->first();

        if (! is_int($articleId)) {
            throw new LogicException("Wiki launch article {$english->slug} has an invalid owner.");
        }

        return WikiArticle::query()->lockForUpdate()->findOrFail($articleId);
    }

    private function assertArticleMatches(
        WikiArticle $article,
        WikiLaunchArticle $definition,
    ): void {
        $englishSlug = $definition->translation('en')->slug;

        if (
            $article->content_type !== $definition->contentType
            || $article->status !== WikiArticleStatus::PUBLISHED
            || $article->is_featured !== $definition->featured
            || $article->sort_order !== $definition->sortOrder
            || $article->published_at === null
            || $article->published_at->isFuture()
        ) {
            throw new LogicException("Wiki launch article {$englishSlug} conflicts with reviewed content.");
        }

        $translations = WikiArticleTranslation::query()
            ->where('article_id', $article->id)
            ->get()
            ->keyBy('locale');

        if ($translations->count() !== count($definition->translations)) {
            throw new LogicException("Wiki launch article {$englishSlug} has an unexpected translation set.");
        }

        foreach ($definition->translations as $expected) {
            $translation = $translations->get($expected->locale);

            if (
                ! $translation instanceof WikiArticleTranslation
                || $translation->title !== $expected->title
                || $translation->slug !== $expected->slug
                || $translation->summary !== $expected->summary
                || $translation->source_markdown !== $expected->sourceMarkdown
            ) {
                throw new LogicException("Wiki launch article {$englishSlug} conflicts with reviewed content.");
            }
        }

        $actualCategoryKeys = DB::table('wiki_article_category as pivot')
            ->join('wiki_categories as category', 'category.id', '=', 'pivot.category_id')
            ->where('pivot.article_id', $article->id)
            ->orderBy('pivot.sort_order')
            ->orderBy('category.key')
            ->pluck('category.key')
            ->all();

        if ($actualCategoryKeys !== $definition->categoryKeys) {
            throw new LogicException("Wiki launch article {$englishSlug} has conflicting categories.");
        }
    }
}
