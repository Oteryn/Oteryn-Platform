<?php

namespace App\Wiki\Http\Admin;

use App\Admin\AdminAuthorization;
use App\Admin\AdminPermission;
use App\Identity\Models\Identity;
use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiCategory;
use Illuminate\Contracts\View\View;
use Illuminate\Http\Request;

final readonly class AdminWikiController
{
    public function __construct(private AdminAuthorization $authorization) {}

    public function __invoke(Request $request): View
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $statusCounts = [];

        foreach (WikiArticleStatus::cases() as $status) {
            $statusCounts[$status->value] = WikiArticle::query()->where('status', $status)->count();
        }

        return view('admin.wiki.index', [
            'articleCount' => array_sum($statusCounts),
            'categoryCount' => WikiCategory::query()->count(),
            'statusCounts' => $statusCounts,
            'canManageArticles' => $this->authorization->allows($identity, AdminPermission::MANAGE_WIKI_ARTICLES),
            'canManageCategories' => $this->authorization->allows($identity, AdminPermission::MANAGE_WIKI_CATEGORIES),
            'canPublish' => $this->authorization->allows($identity, AdminPermission::PUBLISH_WIKI),
        ]);
    }
}
