<?php

namespace App\Http\Controllers\Cms;

use App\Cms\PublicNewsQuery;
use Illuminate\Contracts\View\View;

final class PublicNewsController
{
    public function __construct(private readonly PublicNewsQuery $news) {}

    public function index(): View
    {
        return view('news.index', [
            'posts' => $this->news->published(),
        ]);
    }

    public function show(string $slug): View
    {
        $post = $this->news->findPublishedBySlug($slug);

        abort_if($post === null, 404);

        return view('news.show', ['post' => $post]);
    }
}
