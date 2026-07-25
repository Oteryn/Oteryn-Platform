<?php

namespace App\Cms\Actions;

use App\Audit\AdminAuditRecorder;
use App\Cms\Editorial\EditorialContentType;
use App\Cms\Editorial\EditorialTranslationResolver;
use App\Cms\Models\EditorialTranslation;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
use DateTimeInterface;
use Illuminate\Support\Facades\DB;
use Illuminate\Validation\ValidationException;

final readonly class SaveEditorialTranslation
{
    public function __construct(
        private AdminAuditRecorder $audit,
        private EditorialTranslationResolver $resolver,
    ) {}

    public function execute(
        Identity $actor,
        EditorialContentType $type,
        int $contentId,
        DateTimeInterface $sourceUpdatedAt,
        string $locale,
        ?string $title,
        ?string $body,
        ?string $actionLabel,
        ?string $publishedAt,
    ): ?EditorialTranslation {
        if ($locale !== 'pl') {
            throw ValidationException::withMessages([
                'locale' => 'Only Polish editorial translations are supported by this workflow.',
            ]);
        }

        $title = $this->blankToNull($title);
        $body = $this->blankToNull($body);
        $actionLabel = $this->blankToNull($actionLabel);

        if ($title === null && $body === null && $actionLabel === null) {
            return DB::transaction(function () use ($actor, $type, $contentId, $locale): ?EditorialTranslation {
                $deleted = EditorialTranslation::query()
                    ->where('content_type', $type->value)
                    ->where('content_id', $contentId)
                    ->where('locale', $locale)
                    ->delete();

                if ($deleted > 0) {
                    $this->audit->record(
                        $actor->id,
                        'cms.translation_deleted',
                        $type->value,
                        (string) $contentId,
                        ['locale' => $locale],
                    );
                }

                return null;
            }, 3);
        }

        return DB::transaction(function () use (
            $actor,
            $type,
            $contentId,
            $sourceUpdatedAt,
            $locale,
            $title,
            $body,
            $actionLabel,
            $publishedAt,
        ): EditorialTranslation {
            $translation = EditorialTranslation::query()->updateOrCreate(
                [
                    'content_type' => $type->value,
                    'content_id' => $contentId,
                    'locale' => $locale,
                ],
                [
                    'title' => $title,
                    'body' => $body,
                    'action_label' => $actionLabel,
                    'source_updated_at' => CarbonImmutable::instance($sourceUpdatedAt),
                    'published_at' => $publishedAt,
                ],
            );

            if ($publishedAt !== null && ! $this->resolver->isComplete($type, $translation)) {
                throw ValidationException::withMessages([
                    'published_at' => 'A translation must be complete before it can be published.',
                ]);
            }

            $this->audit->record(
                $actor->id,
                'cms.translation_saved',
                $type->value,
                (string) $contentId,
                [
                    'locale' => $locale,
                    'complete' => $this->resolver->isComplete($type, $translation),
                    'published' => $translation->published_at !== null,
                ],
            );

            return $translation;
        }, 3);
    }

    private function blankToNull(?string $value): ?string
    {
        if ($value === null) {
            return null;
        }

        $trimmed = trim($value);

        return $trimmed === '' ? null : $trimmed;
    }
}
