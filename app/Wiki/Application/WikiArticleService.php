<?php

namespace App\Wiki\Application;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Wiki\Domain\Exceptions\StaleWikiEdit;
use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Domain\WikiContentRules;
use App\Wiki\Domain\WikiLocale;
use App\Wiki\Domain\WikiTranslationInput;
use App\Wiki\Infrastructure\Audit\WikiAuditAction;
use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiArticleTranslation;
use App\Wiki\Infrastructure\Models\WikiRevision;
use DomainException;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;

final readonly class WikiArticleService
{
    public function __construct(
        private WikiAuthorization $authorization,
        private AdminAuditRecorder $audit,
    ) {}

    /**
     * @param  list<WikiTranslationInput>  $translations
     */
    public function create(
        Identity $actor,
        string $contentType,
        array $translations,
        ?string $changeNote = null,
    ): WikiArticle {
        $this->authorization->assertCanManageArticles($actor);
        WikiContentRules::assertContentType($contentType);
        $this->assertTranslationSet($translations);

        return DB::transaction(function () use ($actor, $contentType, $translations, $changeNote): WikiArticle {
            $article = WikiArticle::query()->create([
                'content_type' => $contentType,
                'status' => WikiArticleStatus::DRAFT,
                'author_identity_id' => $actor->id,
                'last_editor_identity_id' => $actor->id,
                'lock_version' => 1,
            ]);

            foreach ($translations as $input) {
                $this->assertSlugAvailable($input->locale, $input->slug, null);
                $translation = $this->saveTranslation($article->id, $input);
                $this->appendRevision($article, $translation, $actor, $changeNote, null);
            }

            $this->audit->record(
                $actor->id,
                WikiAuditAction::ARTICLE_CREATED,
                'wiki_article',
                (string) $article->id,
                [
                    'status' => $article->status->value,
                    'version' => $article->lock_version,
                    'locales' => $this->localeList($translations),
                ],
            );

            return $article;
        }, 3);
    }

    /**
     * @param  list<WikiTranslationInput>  $translations
     */
    public function update(
        Identity $actor,
        WikiArticle $article,
        int $expectedVersion,
        string $contentType,
        array $translations,
        ?string $changeNote = null,
    ): WikiArticle {
        $this->authorization->assertCanManageArticles($actor);
        WikiContentRules::assertContentType($contentType);
        $this->assertTranslationSet($translations);

        return DB::transaction(function () use (
            $actor,
            $article,
            $expectedVersion,
            $contentType,
            $translations,
            $changeNote,
        ): WikiArticle {
            $current = $this->lockArticle($article->id);
            $this->assertVersion($current, $expectedVersion);
            $this->assertEditable($current);

            $current->forceFill([
                'content_type' => $contentType,
                'last_editor_identity_id' => $actor->id,
                'lock_version' => $current->lock_version + 1,
            ])->save();

            foreach ($translations as $input) {
                $this->assertSlugAvailable($input->locale, $input->slug, $current->id);
                $translation = $this->saveTranslation($current->id, $input);
                $this->appendRevision($current, $translation, $actor, $changeNote, null);
            }

            $this->audit->record(
                $actor->id,
                WikiAuditAction::ARTICLE_UPDATED,
                'wiki_article',
                (string) $current->id,
                [
                    'status' => $current->status->value,
                    'version' => $current->lock_version,
                    'locales' => $this->localeList($translations),
                ],
            );

            return $current;
        }, 3);
    }

    public function submitForReview(Identity $actor, WikiArticle $article, int $expectedVersion): WikiArticle
    {
        $this->authorization->assertCanManageArticles($actor);

        return $this->transition(
            $actor,
            $article,
            $expectedVersion,
            WikiArticleStatus::IN_REVIEW,
            WikiAuditAction::ARTICLE_SUBMITTED_FOR_REVIEW,
        );
    }

    public function returnToDraft(Identity $actor, WikiArticle $article, int $expectedVersion): WikiArticle
    {
        $this->authorization->assertCanManageArticles($actor);

        return $this->transition(
            $actor,
            $article,
            $expectedVersion,
            WikiArticleStatus::DRAFT,
            WikiAuditAction::ARTICLE_RETURNED_TO_DRAFT,
        );
    }

    public function publish(Identity $actor, WikiArticle $article, int $expectedVersion): WikiArticle
    {
        $this->authorization->assertCanPublish($actor);

        return DB::transaction(function () use ($actor, $article, $expectedVersion): WikiArticle {
            $current = $this->lockArticle($article->id);
            $this->assertVersion($current, $expectedVersion);
            $current->status->assertCanTransitionTo(WikiArticleStatus::PUBLISHED);
            $this->assertRequiredPublishedTranslations($current);

            $current->forceFill([
                'status' => WikiArticleStatus::PUBLISHED,
                'publisher_identity_id' => $actor->id,
                'published_at' => now(),
                'lock_version' => $current->lock_version + 1,
            ])->save();

            $this->auditArticleState($actor, $current, WikiAuditAction::ARTICLE_PUBLISHED);

            return $current;
        }, 3);
    }

    public function unpublish(Identity $actor, WikiArticle $article, int $expectedVersion): WikiArticle
    {
        $this->authorization->assertCanPublish($actor);

        return DB::transaction(function () use ($actor, $article, $expectedVersion): WikiArticle {
            $current = $this->lockArticle($article->id);
            $this->assertVersion($current, $expectedVersion);
            $current->status->assertCanTransitionTo(WikiArticleStatus::DRAFT);

            $current->forceFill([
                'status' => WikiArticleStatus::DRAFT,
                'published_at' => null,
                'last_editor_identity_id' => $actor->id,
                'lock_version' => $current->lock_version + 1,
            ])->save();

            $this->auditArticleState($actor, $current, WikiAuditAction::ARTICLE_UNPUBLISHED);

            return $current;
        }, 3);
    }

    public function archive(Identity $actor, WikiArticle $article, int $expectedVersion): WikiArticle
    {
        $this->authorization->assertCanPublish($actor);

        return DB::transaction(function () use ($actor, $article, $expectedVersion): WikiArticle {
            $current = $this->lockArticle($article->id);
            $this->assertVersion($current, $expectedVersion);
            $current->status->assertCanTransitionTo(WikiArticleStatus::ARCHIVED);

            $current->forceFill([
                'status' => WikiArticleStatus::ARCHIVED,
                'published_at' => null,
                'last_editor_identity_id' => $actor->id,
                'lock_version' => $current->lock_version + 1,
            ])->save();

            $this->auditArticleState($actor, $current, WikiAuditAction::ARTICLE_ARCHIVED);

            return $current;
        }, 3);
    }

    public function restoreRevision(
        Identity $actor,
        WikiArticle $article,
        int $expectedVersion,
        WikiRevision $revision,
        ?string $changeNote = null,
    ): WikiArticle {
        $this->authorization->assertCanPublish($actor);

        return DB::transaction(function () use (
            $actor,
            $article,
            $expectedVersion,
            $revision,
            $changeNote,
        ): WikiArticle {
            $current = $this->lockArticle($article->id);
            $this->assertVersion($current, $expectedVersion);
            $this->assertEditable($current);

            $source = WikiRevision::query()->findOrFail($revision->id);

            if ($source->article_id !== $current->id) {
                throw new InvalidArgumentException('The selected Wiki revision belongs to another article.');
            }

            $this->assertSlugAvailable($source->locale, $source->slug, $current->id);
            $translation = $this->saveTranslation(
                $current->id,
                new WikiTranslationInput(
                    $source->locale,
                    $source->title,
                    $source->slug,
                    $source->summary,
                    $source->source_markdown,
                ),
            );

            $current->forceFill([
                'last_editor_identity_id' => $actor->id,
                'lock_version' => $current->lock_version + 1,
            ])->save();

            $restored = $this->appendRevision(
                $current,
                $translation,
                $actor,
                $changeNote,
                $source->id,
            );

            $this->audit->record(
                $actor->id,
                WikiAuditAction::REVISION_RESTORED,
                'wiki_article',
                (string) $current->id,
                [
                    'locale' => $source->locale,
                    'version' => $current->lock_version,
                    'revision_number' => $restored->revision_number,
                    'source_revision_id' => $source->id,
                ],
            );

            return $current;
        }, 3);
    }

    private function transition(
        Identity $actor,
        WikiArticle $article,
        int $expectedVersion,
        WikiArticleStatus $target,
        string $auditAction,
    ): WikiArticle {
        return DB::transaction(function () use ($actor, $article, $expectedVersion, $target, $auditAction): WikiArticle {
            $current = $this->lockArticle($article->id);
            $this->assertVersion($current, $expectedVersion);
            $current->status->assertCanTransitionTo($target);

            $current->forceFill([
                'status' => $target,
                'last_editor_identity_id' => $actor->id,
                'lock_version' => $current->lock_version + 1,
            ])->save();

            $this->auditArticleState($actor, $current, $auditAction);

            return $current;
        }, 3);
    }

    private function lockArticle(int $articleId): WikiArticle
    {
        return WikiArticle::query()->lockForUpdate()->findOrFail($articleId);
    }

    private function assertVersion(WikiArticle $article, int $expectedVersion): void
    {
        if ($article->lock_version !== $expectedVersion) {
            throw new StaleWikiEdit('The Wiki article changed after this edit began.');
        }
    }

    private function assertEditable(WikiArticle $article): void
    {
        if (! $article->status->isEditable()) {
            throw new DomainException('Published or archived Wiki articles cannot be edited directly.');
        }
    }

    /**
     * @param  list<WikiTranslationInput>  $translations
     */
    private function assertTranslationSet(array $translations): void
    {
        if ($translations === []) {
            throw new InvalidArgumentException('At least one Wiki article translation is required.');
        }

        $locales = array_map(
            static fn (WikiTranslationInput $input): string => $input->locale,
            $translations,
        );

        if (count($locales) !== count(array_unique($locales))) {
            throw new InvalidArgumentException('A Wiki article translation locale may appear only once per write.');
        }
    }

    private function assertSlugAvailable(string $locale, string $slug, ?int $exceptArticleId): void
    {
        $query = WikiArticleTranslation::query()
            ->where('locale', $locale)
            ->where('slug', $slug);

        if ($exceptArticleId !== null) {
            $query->where('article_id', '!=', $exceptArticleId);
        }

        if ($query->exists()) {
            throw new DomainException('The localized Wiki article slug is already in use.');
        }
    }

    private function saveTranslation(int $articleId, WikiTranslationInput $input): WikiArticleTranslation
    {
        $translation = WikiArticleTranslation::query()->firstOrNew([
            'article_id' => $articleId,
            'locale' => $input->locale,
        ]);

        $translation->fill([
            'title' => $input->title,
            'slug' => $input->slug,
            'summary' => $input->summary,
            'source_markdown' => $input->sourceMarkdown,
        ])->save();

        return $translation;
    }

    private function appendRevision(
        WikiArticle $article,
        WikiArticleTranslation $translation,
        Identity $actor,
        ?string $changeNote,
        ?int $sourceRevisionId,
    ): WikiRevision {
        $latestNumber = WikiRevision::query()
            ->where('article_id', $article->id)
            ->where('locale', $translation->locale)
            ->max('revision_number');

        $nextNumber = is_int($latestNumber)
            ? $latestNumber + 1
            : (is_numeric($latestNumber) ? (int) $latestNumber + 1 : 1);

        return WikiRevision::query()->create([
            'article_id' => $article->id,
            'locale' => $translation->locale,
            'revision_number' => $nextNumber,
            'article_version' => $article->lock_version,
            'title' => $translation->title,
            'slug' => $translation->slug,
            'summary' => $translation->summary,
            'source_markdown' => $translation->source_markdown,
            'editor_identity_id' => $actor->id,
            'change_note' => $changeNote,
            'source_revision_id' => $sourceRevisionId,
        ]);
    }

    private function assertRequiredPublishedTranslations(WikiArticle $article): void
    {
        $translations = WikiArticleTranslation::query()
            ->where('article_id', $article->id)
            ->whereIn('locale', WikiLocale::values())
            ->get()
            ->keyBy('locale');

        foreach (WikiLocale::values() as $locale) {
            $translation = $translations->get($locale);

            if (! $translation instanceof WikiArticleTranslation) {
                throw new DomainException("Wiki publication requires a complete {$locale} translation.");
            }

            WikiContentRules::assertPublishableArticleTranslation(
                $translation->title,
                $translation->slug,
                $translation->summary,
                $translation->source_markdown,
            );
        }
    }

    /**
     * @param  list<WikiTranslationInput>  $translations
     */
    private function localeList(array $translations): string
    {
        $locales = array_map(
            static fn (WikiTranslationInput $input): string => $input->locale,
            $translations,
        );
        sort($locales, SORT_STRING);

        return implode(',', $locales);
    }

    private function auditArticleState(Identity $actor, WikiArticle $article, string $action): void
    {
        $this->audit->record(
            $actor->id,
            $action,
            'wiki_article',
            (string) $article->id,
            [
                'status' => $article->status->value,
                'version' => $article->lock_version,
            ],
        );
    }
}
