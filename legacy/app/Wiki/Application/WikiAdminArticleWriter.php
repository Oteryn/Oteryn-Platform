<?php

namespace App\Wiki\Application;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Wiki\Application\Media\WikiMediaReferenceSynchronizer;
use App\Wiki\Domain\WikiTranslationInput;
use App\Wiki\Infrastructure\Audit\WikiAuditAction;
use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiCategory;
use App\Wiki\Infrastructure\Models\WikiRevision;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;

final readonly class WikiAdminArticleWriter
{
    public function __construct(
        private WikiArticleService $articles,
        private AdminAuditRecorder $audit,
        private WikiMediaReferenceSynchronizer $media,
    ) {}

    /**
     * @param  list<WikiTranslationInput>  $translations
     * @param  list<int>  $categoryIds
     */
    public function create(
        Identity $actor,
        string $contentType,
        array $translations,
        ?string $changeNote,
        bool $featured,
        int $sortOrder,
        array $categoryIds,
    ): WikiArticle {
        return DB::transaction(function () use (
            $actor,
            $contentType,
            $translations,
            $changeNote,
            $featured,
            $sortOrder,
            $categoryIds,
        ): WikiArticle {
            $article = $this->articles->create($actor, $contentType, $translations, $changeNote);
            $this->applyPresentation($actor, $article, $featured, $sortOrder, $categoryIds);
            $this->media->synchronize($article);

            return $article->refresh();
        }, 3);
    }

    /**
     * @param  list<WikiTranslationInput>  $translations
     * @param  list<int>  $categoryIds
     */
    public function update(
        Identity $actor,
        WikiArticle $article,
        int $expectedVersion,
        string $contentType,
        array $translations,
        ?string $changeNote,
        bool $featured,
        int $sortOrder,
        array $categoryIds,
    ): WikiArticle {
        return DB::transaction(function () use (
            $actor,
            $article,
            $expectedVersion,
            $contentType,
            $translations,
            $changeNote,
            $featured,
            $sortOrder,
            $categoryIds,
        ): WikiArticle {
            $current = $this->articles->update(
                $actor,
                $article,
                $expectedVersion,
                $contentType,
                $translations,
                $changeNote,
            );
            $this->applyPresentation($actor, $current, $featured, $sortOrder, $categoryIds);
            $this->media->synchronize($current);

            return $current->refresh();
        }, 3);
    }

    public function restoreRevision(
        Identity $actor,
        WikiArticle $article,
        int $expectedVersion,
        WikiRevision $revision,
        ?string $changeNote,
    ): WikiArticle {
        return DB::transaction(function () use (
            $actor,
            $article,
            $expectedVersion,
            $revision,
            $changeNote,
        ): WikiArticle {
            $current = $this->articles->restoreRevision(
                $actor,
                $article,
                $expectedVersion,
                $revision,
                $changeNote,
            );
            $this->media->synchronize($current);

            return $current->refresh();
        }, 3);
    }

    /** @param list<int> $categoryIds */
    private function applyPresentation(
        Identity $actor,
        WikiArticle $article,
        bool $featured,
        int $sortOrder,
        array $categoryIds,
    ): void {
        if ($sortOrder < 0) {
            throw new InvalidArgumentException('Wiki article sort order cannot be negative.');
        }

        $normalizedCategoryIds = [];

        foreach ($categoryIds as $categoryId) {
            if ($categoryId < 1) {
                throw new InvalidArgumentException('Wiki category identifiers must be positive integers.');
            }

            $normalizedCategoryIds[$categoryId] = $categoryId;
        }

        $normalizedCategoryIds = array_values($normalizedCategoryIds);

        if (
            $normalizedCategoryIds !== []
            && WikiCategory::query()->whereIn('id', $normalizedCategoryIds)->count() !== count($normalizedCategoryIds)
        ) {
            throw new InvalidArgumentException('One or more selected Wiki categories do not exist.');
        }

        $article->forceFill([
            'is_featured' => $featured,
            'sort_order' => $sortOrder,
        ])->save();

        DB::table('wiki_article_category')->where('article_id', $article->id)->delete();

        if ($normalizedCategoryIds !== []) {
            DB::table('wiki_article_category')->insert(array_map(
                static fn (int $categoryId, int $position): array => [
                    'article_id' => $article->id,
                    'category_id' => $categoryId,
                    'sort_order' => $position,
                ],
                $normalizedCategoryIds,
                array_keys($normalizedCategoryIds),
            ));
        }

        $this->audit->record(
            $actor->id,
            WikiAuditAction::ARTICLE_PRESENTATION_UPDATED,
            'wiki_article',
            (string) $article->id,
            [
                'featured' => $featured,
                'sort_order' => $sortOrder,
                'category_count' => count($normalizedCategoryIds),
                'version' => $article->lock_version,
            ],
        );
    }
}
