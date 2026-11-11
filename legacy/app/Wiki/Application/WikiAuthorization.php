<?php

namespace App\Wiki\Application;

use App\Admin\AdminAuthorization;
use App\Admin\AdminPermission;
use App\Identity\Models\Identity;
use Illuminate\Auth\Access\AuthorizationException;

final readonly class WikiAuthorization
{
    public function __construct(private AdminAuthorization $authorization) {}

    public function assertCanAccess(Identity $actor): void
    {
        $this->assertAllowed($actor, AdminPermission::WIKI_ACCESS);
    }

    public function assertCanManageArticles(Identity $actor): void
    {
        $this->assertAllowed($actor, AdminPermission::MANAGE_WIKI_ARTICLES);
    }

    public function assertCanManageCategories(Identity $actor): void
    {
        $this->assertAllowed($actor, AdminPermission::MANAGE_WIKI_CATEGORIES);
    }

    public function assertCanPublish(Identity $actor): void
    {
        $this->assertAllowed($actor, AdminPermission::PUBLISH_WIKI);
    }

    private function assertAllowed(Identity $actor, string $permission): void
    {
        if (! $this->authorization->allows($actor, $permission)) {
            throw new AuthorizationException('This Identity does not have the required Wiki permission.');
        }
    }
}
