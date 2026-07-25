<?php

namespace App\Cms\Editorial;

use App\Cms\Models\ManagedPage;
use DateTimeInterface;

final class EditorialPageQuery
{
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

        return new EditorialPageResult($key, EditorialPageState::Published, $page);
    }
}
