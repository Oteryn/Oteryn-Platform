<?php

namespace App\Http\Controllers\Admin;

use App\Cms\Actions\SaveManagedPage;
use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Models\ManagedPage;
use App\Http\Requests\Admin\AdminManagedPageRequest;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;

final class AdminManagedPageController
{
    public function index(): View
    {
        return view('admin.pages.index', [
            'pages' => ManagedPage::query()
                ->whereNotIn('slug', EditorialPageKey::managedPageSlugs())
                ->orderByDesc('updated_at')
                ->orderByDesc('id')
                ->paginate(25),
        ]);
    }

    public function create(): View
    {
        return view('admin.pages.form', ['page' => null]);
    }

    public function store(AdminManagedPageRequest $request, SaveManagedPage $save): RedirectResponse
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $page = $save->execute(
            $identity,
            null,
            $request->string('slug')->toString(),
            $request->string('title')->toString(),
            $request->string('body')->toString(),
            $request->filled('published_at') ? $request->string('published_at')->toString() : null,
        );

        return redirect()->route('admin.pages.edit', $page)->with('status', 'Managed page saved.');
    }

    public function edit(ManagedPage $managedPage): View
    {
        abort_if(EditorialPageKey::fromManagedPageSlug($managedPage->slug) !== null, 404);

        return view('admin.pages.form', ['page' => $managedPage]);
    }

    public function update(
        AdminManagedPageRequest $request,
        ManagedPage $managedPage,
        SaveManagedPage $save,
    ): RedirectResponse {
        abort_if(EditorialPageKey::fromManagedPageSlug($managedPage->slug) !== null, 404);

        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $save->execute(
            $identity,
            $managedPage,
            $request->string('slug')->toString(),
            $request->string('title')->toString(),
            $request->string('body')->toString(),
            $request->filled('published_at') ? $request->string('published_at')->toString() : null,
        );

        return redirect()->route('admin.pages.edit', $managedPage)->with('status', 'Managed page saved.');
    }
}
