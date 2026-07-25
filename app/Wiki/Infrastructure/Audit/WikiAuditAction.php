<?php

namespace App\Wiki\Infrastructure\Audit;

final class WikiAuditAction
{
    public const ARTICLE_CREATED = 'wiki.article_created';

    public const ARTICLE_UPDATED = 'wiki.article_updated';

    public const ARTICLE_SUBMITTED_FOR_REVIEW = 'wiki.article_submitted_for_review';

    public const ARTICLE_RETURNED_TO_DRAFT = 'wiki.article_returned_to_draft';

    public const ARTICLE_PUBLISHED = 'wiki.article_published';

    public const ARTICLE_UNPUBLISHED = 'wiki.article_unpublished';

    public const ARTICLE_ARCHIVED = 'wiki.article_archived';

    public const REVISION_RESTORED = 'wiki.revision_restored';

    public const CATEGORY_CREATED = 'wiki.category_created';

    public const CATEGORY_UPDATED = 'wiki.category_updated';
}
