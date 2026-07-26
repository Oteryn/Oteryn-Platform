<?php

namespace Tests\Feature\EditorialMedia;

use App\Admin\AdminRoleManager;

final class WikiEditorialMediaSecurityTest extends EditorialMediaTestCase
{
    public function test_media_existence_and_media_management_authority_do_not_grant_wiki_delivery_or_picker_access(): void
    {
        $actor = $this->createIdentity('media-manager-without-wiki@example.test');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
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
}
