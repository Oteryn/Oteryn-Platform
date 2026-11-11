<?php

namespace App\Wiki\Domain;

use App\Wiki\Domain\Exceptions\InvalidWikiLifecycleTransition;

enum WikiArticleStatus: string
{
    case DRAFT = 'draft';
    case IN_REVIEW = 'in_review';
    case PUBLISHED = 'published';
    case ARCHIVED = 'archived';

    public function canTransitionTo(self $target): bool
    {
        return match ($this) {
            self::DRAFT => in_array($target, [self::IN_REVIEW, self::ARCHIVED], true),
            self::IN_REVIEW => in_array($target, [self::DRAFT, self::PUBLISHED, self::ARCHIVED], true),
            self::PUBLISHED => in_array($target, [self::DRAFT, self::ARCHIVED], true),
            self::ARCHIVED => false,
        };
    }

    public function assertCanTransitionTo(self $target): void
    {
        if (! $this->canTransitionTo($target)) {
            throw new InvalidWikiLifecycleTransition(
                "Wiki article cannot transition from {$this->value} to {$target->value}.",
            );
        }
    }

    public function isEditable(): bool
    {
        return in_array($this, [self::DRAFT, self::IN_REVIEW], true);
    }
}
