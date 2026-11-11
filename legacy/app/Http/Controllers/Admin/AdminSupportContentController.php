<?php

namespace App\Http\Controllers\Admin;

use App\Cms\Actions\SaveManagedPage;
use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\ManagedPageLegalVersion;
use App\Http\Requests\Admin\AdminSupportContentRequest;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;

final class AdminSupportContentController
{
    public function index(): View
    {
        return view('admin.support-content.index', [
            'keys' => EditorialPageKey::cases(),
            'pages' => ManagedPage::query()
                ->whereIn('slug', EditorialPageKey::managedPageSlugs())
                ->get()
                ->keyBy('slug'),
        ]);
    }

    public function edit(string $editorialPageKey): View
    {
        $key = $this->resolveKey($editorialPageKey);
        $page = ManagedPage::query()->where('slug', $key->managedPageSlug())->first();

        return view('admin.support-content.form', [
            'key' => $key,
            'page' => $page,
            'legalVersions' => $page === null || ! $key->isLegal()
                ? collect()
                : ManagedPageLegalVersion::query()
                    ->where('managed_page_id', $page->id)
                    ->orderByDesc('effective_date')
                    ->orderByDesc('id')
                    ->get(),
        ]);
    }

    public function update(
        AdminSupportContentRequest $request,
        string $editorialPageKey,
        SaveManagedPage $save,
    ): RedirectResponse {
        $key = $this->resolveKey($editorialPageKey);
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $page = ManagedPage::query()->where('slug', $key->managedPageSlug())->first();

        $saved = $save->execute(
            $identity,
            $page,
            $key->managedPageSlug(),
            $request->string('title')->toString(),
            $request->string('body')->toString(),
            $request->filled('published_at') ? $request->string('published_at')->toString() : null,
            $key->isLegal() && $request->filled('legal_version')
                ? $request->string('legal_version')->toString()
                : null,
            $key->isLegal() && $request->filled('legal_effective_date')
                ? $request->string('legal_effective_date')->toString()
                : null,
            'support.content',
        );

        return redirect()
            ->route('admin.support-content.edit', ['editorialPageKey' => $key->value])
            ->with('status', $saved->wasRecentlyCreated ? 'Editorial page created.' : 'Editorial page saved.');
    }

    private function resolveKey(string $value): EditorialPageKey
    {
        $key = EditorialPageKey::tryFrom($value);

        if ($key === null) {
            abort(404);
        }

        return $key;
    }
}
