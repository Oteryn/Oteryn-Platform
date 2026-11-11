<?php

namespace App\Http\Controllers\Support;

use App\Cms\Editorial\EditorialPageKey;
use App\Support\PublicEditorialPage;
use Illuminate\Http\Request;
use Illuminate\Http\Response;

final class EditorialPageController
{
    public function __construct(private readonly PublicEditorialPage $page) {}

    public function __invoke(Request $request): Response
    {
        $key = EditorialPageKey::tryFrom((string) $request->route('editorialPageKey'));

        if ($key === null || $key->isSupportGuidance()) {
            abort(404);
        }

        return $this->page->render($key, false);
    }
}
