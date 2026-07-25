<?php

namespace App\Cms\Actions;

use App\Audit\AdminAuditRecorder;
use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\ManagedPageLegalVersion;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Illuminate\Validation\ValidationException;

final class SaveManagedPage
{
    public function __construct(private readonly AdminAuditRecorder $audit) {}

    public function execute(
        Identity $actor,
        ?ManagedPage $page,
        string $slug,
        string $title,
        string $body,
        ?string $publishedAt,
        ?string $legalVersion = null,
        ?string $legalEffectiveDate = null,
        string $auditActionPrefix = 'cms.page',
    ): ManagedPage {
        $editorialKey = EditorialPageKey::fromManagedPageSlug($slug);

        if ($editorialKey === null || ! $editorialKey->isLegal()) {
            $legalVersion = null;
            $legalEffectiveDate = null;
        }

        if (
            $editorialKey?->isLegal()
            && $publishedAt !== null
            && ($legalVersion === null || $legalEffectiveDate === null)
        ) {
            throw ValidationException::withMessages([
                'legal_version' => 'Published legal documents require a version and effective date.',
            ]);
        }

        return DB::transaction(function () use (
            $actor,
            $page,
            $slug,
            $title,
            $body,
            $publishedAt,
            $legalVersion,
            $legalEffectiveDate,
            $auditActionPrefix,
            $editorialKey,
        ): ManagedPage {
            $created = $page === null;
            $page ??= new ManagedPage;
            $page->fill([
                'slug' => $slug,
                'title' => $title,
                'body' => $body,
                'legal_version' => $legalVersion,
                'legal_effective_date' => $legalEffectiveDate,
                'published_at' => $publishedAt,
            ]);
            $page->save();

            if ($editorialKey?->isLegal() && $page->published_at !== null) {
                $this->preserveLegalVersion($page);
            }

            /** @var array<string, bool|int|string|null> $metadata */
            $metadata = [
                'slug' => $page->slug,
                'published' => $page->published_at !== null,
            ];

            if ($editorialKey !== null) {
                $metadata['editorial_key'] = $editorialKey->value;
            }

            if ($page->legal_version !== null) {
                $metadata['legal_version'] = $page->legal_version;
            }

            if ($page->legal_effective_date !== null) {
                $metadata['legal_effective_date'] = $page->legal_effective_date->format('Y-m-d');
            }

            $this->audit->record(
                $actor->id,
                $auditActionPrefix.($created ? '_created' : '_updated'),
                'managed_page',
                (string) $page->id,
                $metadata,
            );

            return $page;
        }, 3);
    }

    private function preserveLegalVersion(ManagedPage $page): void
    {
        if (
            $page->legal_version === null
            || $page->legal_effective_date === null
            || $page->published_at === null
        ) {
            return;
        }

        $existing = ManagedPageLegalVersion::query()
            ->where('managed_page_id', $page->id)
            ->where('version', $page->legal_version)
            ->lockForUpdate()
            ->first();

        if ($existing === null) {
            ManagedPageLegalVersion::query()->create([
                'managed_page_id' => $page->id,
                'version' => $page->legal_version,
                'effective_date' => $page->legal_effective_date,
                'title' => $page->title,
                'body' => $page->body,
                'published_at' => $page->published_at,
            ]);

            return;
        }

        if (
            $existing->title !== $page->title
            || $existing->body !== $page->body
            || $existing->effective_date->format('Y-m-d') !== $page->legal_effective_date->format('Y-m-d')
        ) {
            throw ValidationException::withMessages([
                'legal_version' => 'This published legal version is immutable. Choose a new version before changing its content or effective date.',
            ]);
        }
    }
}
