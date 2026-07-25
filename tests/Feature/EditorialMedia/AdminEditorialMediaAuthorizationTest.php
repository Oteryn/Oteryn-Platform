<?php

namespace Tests\Feature\EditorialMedia;

use App\Admin\AdminRoleManager;
use App\EditorialMedia\Application\Actions\DeleteEditorialImage;
use App\EditorialMedia\Application\Actions\StoreEditorialImage;
use App\EditorialMedia\Application\EditorialMediaReferenceManager;
use App\EditorialMedia\Domain\EditorialMediaConsumer;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use GdImage;
use Illuminate\Foundation\Http\Middleware\ValidateCsrfToken;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Http\UploadedFile;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Storage;
use LogicException;
use RuntimeException;
use Tests\TestCase;

final class AdminEditorialMediaAuthorizationTest extends EditorialMediaTestCase
{
    public function test_media_permission_is_explicitly_granted_only_to_content_and_platform_administrator_roles(): void
    {
        $permissionId = DB::table('admin_permissions')->where('key', 'media.manage')->value('id');
        self::assertTrue(is_int($permissionId) || (is_string($permissionId) && ctype_digit($permissionId)));

        $roleKeys = DB::table('admin_role_permissions')
            ->join('admin_roles', 'admin_roles.id', '=', 'admin_role_permissions.role_id')
            ->where('admin_role_permissions.permission_id', (int) $permissionId)
            ->orderBy('admin_roles.key')
            ->pluck('admin_roles.key')
            ->all();

        self::assertSame(['content_editor', 'platform_admin'], $roleKeys);
    }

    public function test_permission_and_confirmed_mfa_are_both_required(): void
    {
        $securityActor = $this->createIdentity('media-security-only@example.com');
        $this->assignRole($securityActor, AdminRoleManager::SECURITY_ADMIN);
        $this->actingAsCurrent($securityActor);

        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('denied.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => 'Denied by permission.',
        ])->assertForbidden();

        $editorWithoutMfa = $this->createIdentity('media-no-mfa@example.com', false);
        $this->assignRole($editorWithoutMfa, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($editorWithoutMfa);

        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('denied-mfa.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => 'Denied by MFA.',
        ])->assertForbidden();

        $this->assertDatabaseCount('editorial_media', 0);
    }

    public function test_upload_route_remains_csrf_protected(): void
    {
        $actor = $this->createIdentity('media-csrf-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);
        $this->app->detectEnvironment(static fn (): string => 'production');
        $this->withMiddleware(ValidateCsrfToken::class);

        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('csrf.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => 'CSRF-protected upload.',
        ])->assertStatus(419);

        $this->assertDatabaseCount('editorial_media', 0);
        $media = $this->uploadThroughAction($actor, 'csrf-delete.png');
        $this->delete(route('admin.media.destroy', $media))->assertStatus(419);
        $this->assertDatabaseHas('editorial_media', ['id' => $media->id]);
        Storage::disk('editorial_media')->assertExists($media->storage_path);
    }

    public function test_referenced_media_cannot_be_deleted_and_database_restricts_direct_deletion(): void
    {
        $actor = $this->createIdentity('media-reference-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);
        $media = $this->uploadThroughAction($actor, 'referenced.png');

        $reference = app(EditorialMediaReferenceManager::class)->attach(
            $media,
            EditorialMediaConsumer::WIKI,
            'article:42',
            'hero',
        );

        $this->delete(route('admin.media.destroy', $media))
            ->assertSessionHasErrors('media');

        $this->assertDatabaseHas('editorial_media', ['id' => $media->id]);
        $this->assertDatabaseHas('editorial_media_references', ['id' => $reference->id]);
        Storage::disk('editorial_media')->assertExists($media->storage_path);

        try {
            DB::table('editorial_media')->where('id', $media->id)->delete();
            self::fail('The database must reject deletion while a reference exists.');
        } catch (\Throwable) {
            $this->assertDatabaseHas('editorial_media', ['id' => $media->id]);
        }
    }

    public function test_unreferenced_media_can_be_deleted_with_files_and_bounded_audit(): void
    {
        $actor = $this->createIdentity('media-delete-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);
        $media = $this->uploadThroughAction($actor, 'delete-me.png', 300, 200);
        $thumbnailPath = $media->thumbnail_path;
        self::assertNotNull($thumbnailPath);

        $this->delete(route('admin.media.destroy', $media))
            ->assertRedirect(route('admin.media.index'));

        $this->assertDatabaseMissing('editorial_media', ['id' => $media->id]);
        Storage::disk('editorial_media')->assertMissing($media->storage_path);
        Storage::disk('editorial_media')->assertMissing($thumbnailPath);
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'editorial_media.deleted',
            'target_type' => 'editorial_media',
            'target_id' => (string) $media->id,
        ]);
    }
}
