<?php

namespace App\Http\Controllers\Cms;

use App\Cms\Editorial\EditorialPageKey;
use App\Cms\PublicPageQuery;
use Illuminate\Contracts\View\View;

final class PublicPageController
{
    public function __construct(private readonly PublicPageQuery $pages) {}

    public function show(string $slug): View
    {
        abort_if(EditorialPageKey::fromManagedPageSlug($slug) !== null, 404);

        $page = $this->pages->findPublishedBySlug($slug);

        abort_if($page === null, 404);

        return view('pages.show', ['page' => $page]);
    }
}
