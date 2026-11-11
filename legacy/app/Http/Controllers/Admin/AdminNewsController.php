<?php

namespace App\Http\Controllers\Admin;

use App\Cms\Actions\SaveNewsPost;
use App\Cms\Models\NewsPost;
use App\Http\Requests\Admin\AdminNewsPostRequest;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;

final class AdminNewsController
{
    public function index(): View
    {
        return view('admin.news.index', [
            'posts' => NewsPost::query()
                ->orderByDesc('updated_at')
                ->orderByDesc('id')
                ->paginate(25),
        ]);
    }

    public function create(): View
    {
        return view('admin.news.form', ['post' => null]);
    }

    public function store(AdminNewsPostRequest $request, SaveNewsPost $save): RedirectResponse
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $post = $save->execute(
            $identity,
            null,
            $request->string('slug')->toString(),
            $request->string('title')->toString(),
            $request->string('body')->toString(),
            $request->filled('published_at') ? $request->string('published_at')->toString() : null,
        );

        return redirect()->route('admin.news.edit', $post)->with('status', 'News post saved.');
    }

    public function edit(NewsPost $newsPost): View
    {
        return view('admin.news.form', ['post' => $newsPost]);
    }

    public function update(
        AdminNewsPostRequest $request,
        NewsPost $newsPost,
        SaveNewsPost $save,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $save->execute(
            $identity,
            $newsPost,
            $request->string('slug')->toString(),
            $request->string('title')->toString(),
            $request->string('body')->toString(),
            $request->filled('published_at') ? $request->string('published_at')->toString() : null,
        );

        return redirect()->route('admin.news.edit', $newsPost)->with('status', 'News post saved.');
    }
}
