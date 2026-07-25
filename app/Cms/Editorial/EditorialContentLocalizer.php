<?php

namespace App\Cms\Editorial;

use DateTimeInterface;
use Illuminate\Database\Eloquent\Model;

final readonly class EditorialContentLocalizer
{
    public function __construct(private EditorialTranslationResolver $translations) {}

    /**
     * @param array<string, string> $attributeMap translation attribute => source attribute
     */
    public function localize(Model $source, EditorialContentType $type, string $locale, array $attributeMap): ?Model
    {
        if ($locale === 'en') {
            return $source;
        }

        $updatedAt = $source->getAttribute('updated_at');
        if (! $updatedAt instanceof DateTimeInterface) {
            return null;
        }

        $translation = $this->translations->published(
            $type,
            (int) $source->getKey(),
            $updatedAt,
            $locale,
        );
        if ($translation === null) {
            return null;
        }

        $localized = clone $source;
        foreach ($attributeMap as $translationAttribute => $sourceAttribute) {
            $localized->setAttribute($sourceAttribute, $translation->getAttribute($translationAttribute));
        }

        return $localized;
    }
}
