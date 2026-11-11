<?php

namespace App\Cms\Editorial;

use App\Cms\Models\EditorialTranslation;
use Carbon\CarbonImmutable;
use DateTimeInterface;

final class EditorialTranslationResolver
{
    public function find(EditorialContentType $type, int $contentId, string $locale): ?EditorialTranslation
    {
        if ($locale === 'en') {
            return null;
        }

        return EditorialTranslation::query()
            ->where('content_type', $type->value)
            ->where('content_id', $contentId)
            ->where('locale', $locale)
            ->first();
    }

    public function state(
        EditorialContentType $type,
        int $contentId,
        DateTimeInterface $sourceUpdatedAt,
        string $locale,
        ?DateTimeInterface $readTime = null,
    ): EditorialTranslationState {
        if ($locale === 'en') {
            return EditorialTranslationState::Published;
        }

        $translation = $this->find($type, $contentId, $locale);
        if ($translation === null) {
            return EditorialTranslationState::Missing;
        }

        if (! $this->isComplete($type, $translation)) {
            return EditorialTranslationState::Incomplete;
        }

        if ($translation->source_updated_at->lt(CarbonImmutable::instance($sourceUpdatedAt))) {
            return EditorialTranslationState::Stale;
        }

        $at = CarbonImmutable::instance($readTime ?? now());
        if ($translation->published_at === null || $translation->published_at->isAfter($at)) {
            return EditorialTranslationState::Draft;
        }

        return EditorialTranslationState::Published;
    }

    public function published(
        EditorialContentType $type,
        int $contentId,
        DateTimeInterface $sourceUpdatedAt,
        string $locale,
        ?DateTimeInterface $readTime = null,
    ): ?EditorialTranslation {
        if ($this->state($type, $contentId, $sourceUpdatedAt, $locale, $readTime) !== EditorialTranslationState::Published) {
            return null;
        }

        return $this->find($type, $contentId, $locale);
    }

    public function isComplete(EditorialContentType $type, EditorialTranslation $translation): bool
    {
        if ($type->requiresTitle() && $this->blank($translation->title)) {
            return false;
        }

        return ! $this->blank($translation->body);
    }

    private function blank(?string $value): bool
    {
        return $value === null || trim($value) === '';
    }
}
