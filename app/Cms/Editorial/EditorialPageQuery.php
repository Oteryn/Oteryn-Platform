<?php

namespace App\Cms\Editorial;

use App\Cms\Models\ManagedPage;
use DateTimeInterface;

final readonly class EditorialPageQuery
{
    public function __construct(private EditorialContentLocalizer $localizer) {}

    public function find(EditorialPageKey $key, ?DateTimeInterface $readTime = null): EditorialPageResult
    {
        $readTime ??= now();

        $page = ManagedPage::query()
            ->where('slug', $key->managedPageSlug())
            ->first();

        if ($page === null) {
            return new EditorialPageResult($key, EditorialPageState::Missing, null);
        }

        if ($page->published_at === null || $page->published_at->isAfter($readTime)) {
            return new EditorialPageResult($key, EditorialPageState::Unpublished, null);
        }

        $localized = $this->localizer->localize(
            $page,
            EditorialContentType::ManagedPage,
            app()->getLocale(),
            ['title' => 'title', 'body' => 'body'],
        );

        if (! $localized instanceof ManagedPage) {
            return new EditorialPageResult($key, EditorialPageState::TranslationUnavailable, null);
        }

        return new EditorialPageResult($key, EditorialPageState::Published, $localized);
    }
}
