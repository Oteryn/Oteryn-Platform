<?php

namespace Tests\Feature\EditorialMedia;

use App\Admin\AdminPermission;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;

final class WikiEditorialMediaSecurityTest extends EditorialMediaTestCase
{
    public function test_media_existence_and_media_management_authority_do_not_grant_wiki_delivery_or_picker_access(): void
    {
        $actor = $this->createIdentity('media-manager-without-wiki@example.test');
        $this->assignMediaOnlyRole($actor);
        $media = $this->uploadThroughAction($actor, 'unreferenced.png');
        $this->actingAsCurrent($actor);

        $this->get(route('admin.media.index'))->assertOk();
        $this->get(route('admin.wiki.media.index'))->assertForbidden();
        $unreferenced = $this->get(route('wiki.media', [
            'locale' => 'en',
            'editorialMedia' => $media,
        ]));
        $unreferenced->assertNotFound();
        self::assertStringContainsString(
            'no-store',
            (string) $unreferenced->headers->get('Cache-Control'),
        );
        $this->get(route('legacy.wiki.media', $media))->assertNotFound();
        $this->get('/en/wiki/media/0'.$media->id)->assertNotFound();
        $this->get('/en/wiki/media/999999999999999999999999')->assertNotFound();
    }

    private function assignMediaOnlyRole(Identity $identity): void
    {
        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => 'media_only_test',
            'name' => 'Media-only test role',
            'created_at' => now(),
            'updated_at' => now(),
        ]);
        $permissionId = DB::table('admin_permissions')
            ->where('key', AdminPermission::MANAGE_MEDIA)
            ->value('id');

        self::assertIsInt($permissionId);

        DB::table('admin_role_permissions')->insert([
            'role_id' => $roleId,
            'permission_id' => $permissionId,
        ]);
        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);
    }
}
