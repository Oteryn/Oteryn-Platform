<?php

namespace App\Http\Controllers\Admin;

use App\Announcements\Models\SiteAnnouncement;
use App\Cms\Actions\SaveEditorialTranslation;
use App\Cms\Editorial\EditorialContentType;
use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Editorial\EditorialTranslationResolver;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\NewsPost;
use App\Downloads\Models\ClientRelease;
use App\Http\Requests\Admin\AdminEditorialTranslationRequest;
use App\Identity\Models\Identity;
use DateTimeInterface;
use Illuminate\Contracts\View\View;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Http\RedirectResponse;

final readonly class AdminEditorialTranslationController
{
    public function __construct(private EditorialTranslationResolver $translations) {}

    public function editNews(NewsPost $newsPost): View
    {
        return $this->edit(
            EditorialContentType::NewsPost,
            $newsPost,
            'admin.news.translation.update',
            route('admin.news.edit', ['newsPost' => $newsPost]),
        );
    }

    public function updateNews(
        AdminEditorialTranslationRequest $request,
        NewsPost $newsPost,
        SaveEditorialTranslation $save,
    ): RedirectResponse {
        return $this->update(
            $request,
            EditorialContentType::NewsPost,
            $newsPost,
            $save,
            'admin.news.translation.edit',
            'newsPost',
        );
    }

    public function editPage(ManagedPage $managedPage): View
    {
        $this->assertGenericManagedPage($managedPage);

        return $this->edit(
            EditorialContentType::ManagedPage,
            $managedPage,
            'admin.pages.translation.update',
            route('admin.pages.edit', ['managedPage' => $managedPage]),
        );
    }

    public function updatePage(
        AdminEditorialTranslationRequest $request,
        ManagedPage $managedPage,
        SaveEditorialTranslation $save,
    ): RedirectResponse {
        $this->assertGenericManagedPage($managedPage);

        return $this->update(
            $request,
            EditorialContentType::ManagedPage,
            $managedPage,
            $save,
            'admin.pages.translation.edit',
            'managedPage',
        );
    }

    public function editSupport(ManagedPage $managedPage): View
    {
        $this->assertReservedManagedPage($managedPage);

        return $this->edit(
            EditorialContentType::ManagedPage,
            $managedPage,
            'admin.support-content.translation.update',
            route('admin.support-content.index'),
        );
    }

    public function updateSupport(
        AdminEditorialTranslationRequest $request,
        ManagedPage $managedPage,
        SaveEditorialTranslation $save,
    ): RedirectResponse {
        $this->assertReservedManagedPage($managedPage);

        return $this->update(
            $request,
            EditorialContentType::ManagedPage,
            $managedPage,
            $save,
            'admin.support-content.translation.edit',
            'managedPage',
        );
    }

    public function editAnnouncement(SiteAnnouncement $siteAnnouncement): View
    {
        return $this->edit(
            EditorialContentType::SiteAnnouncement,
            $siteAnnouncement,
            'admin.announcements.translation.update',
            route('admin.announcements.edit', ['siteAnnouncement' => $siteAnnouncement]),
        );
    }

    public function updateAnnouncement(
        AdminEditorialTranslationRequest $request,
        SiteAnnouncement $siteAnnouncement,
        SaveEditorialTranslation $save,
    ): RedirectResponse {
        return $this->update(
            $request,
            EditorialContentType::SiteAnnouncement,
            $siteAnnouncement,
            $save,
            'admin.announcements.translation.edit',
            'siteAnnouncement',
        );
    }

    public function editRelease(ClientRelease $clientRelease): View
    {
        return $this->edit(
            EditorialContentType::ClientRelease,
            $clientRelease,
            'admin.downloads.translation.update',
            route('admin.downloads.edit', ['clientRelease' => $clientRelease]),
        );
    }

    public function updateRelease(
        AdminEditorialTranslationRequest $request,
        ClientRelease $clientRelease,
        SaveEditorialTranslation $save,
    ): RedirectResponse {
        return $this->update(
            $request,
            EditorialContentType::ClientRelease,
            $clientRelease,
            $save,
            'admin.downloads.translation.edit',
            'clientRelease',
        );
    }

    private function assertGenericManagedPage(ManagedPage $page): void
    {
        $slug = $page->getAttribute('slug');
        abort_unless(is_string($slug), 404);
        abort_if(EditorialPageKey::fromManagedPageSlug($slug) !== null, 404);
    }

    private function assertReservedManagedPage(ManagedPage $page): void
    {
        $slug = $page->getAttribute('slug');
        abort_unless(is_string($slug), 404);
        abort_if(EditorialPageKey::fromManagedPageSlug($slug) === null, 404);
    }

    private function edit(EditorialContentType $type, Model $source, string $updateRoute, string $backUrl): View
    {
        $sourceId = $this->sourceId($source);
        $updatedAt = $source->getAttribute('updated_at');
        abort_unless($updatedAt instanceof DateTimeInterface, 500);

        return view('admin.translations.form', [
            'type' => $type,
            'source' => $source,
            'translation' => $this->translations->find($type, $sourceId, 'pl'),
            'translationState' => $this->translations->state($type, $sourceId, $updatedAt, 'pl'),
            'updateRoute' => $updateRoute,
            'backUrl' => $backUrl,
        ]);
    }

    private function update(
        AdminEditorialTranslationRequest $request,
        EditorialContentType $type,
        Model $source,
        SaveEditorialTranslation $save,
        string $editRoute,
        string $routeParameter,
    ): RedirectResponse {
        $actor = $request->user();
        abort_unless($actor instanceof Identity, 403);

        $sourceId = $this->sourceId($source);
        $updatedAt = $source->getAttribute('updated_at');
        abort_unless($updatedAt instanceof DateTimeInterface, 500);

        $save->execute(
            $actor,
            $type,
            $sourceId,
            $updatedAt,
            'pl',
            $request->filled('title') ? $request->string('title')->toString() : null,
            $request->filled('body') ? $request->string('body')->toString() : null,
            $request->filled('action_label') ? $request->string('action_label')->toString() : null,
            $request->filled('published_at') ? $request->string('published_at')->toString() : null,
        );

        return redirect()
            ->route($editRoute, [$routeParameter => $source])
            ->with('status', 'Polish translation saved.');
    }

    private function sourceId(Model $source): int
    {
        $sourceId = $source->getKey();
        abort_unless(is_int($sourceId) || (is_string($sourceId) && ctype_digit($sourceId)), 500);

        return (int) $sourceId;
    }
}
