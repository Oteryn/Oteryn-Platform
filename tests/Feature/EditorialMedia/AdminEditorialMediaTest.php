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

final class AdminEditorialMediaTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        Storage::fake('editorial_media');
        config([
            'editorial_media.disk' => 'editorial_media',
            'editorial_media.max_bytes' => 8 * 1024 * 1024,
            'editorial_media.max_width' => 5000,
            'editorial_media.max_height' => 5000,
            'editorial_media.max_pixels' => 12_000_000,
            'editorial_media.thumbnail_max_dimension' => 100,
            'editorial_media.jpeg_quality' => 88,
            'editorial_media.webp_quality' => 85,
            'editorial_media.png_compression' => 6,
        ]);
    }

    public function test_editorial_media_disk_is_private_and_fail_closed(): void
    {
        self::assertSame(storage_path('app/editorial-media'), config('filesystems.disks.editorial_media.root'));
        self::assertSame('private', config('filesystems.disks.editorial_media.visibility'));
        self::assertFalse(config('filesystems.disks.editorial_media.serve'));
        self::assertTrue(config('filesystems.disks.editorial_media.throw'));
        self::assertTrue(config('filesystems.disks.editorial_media.report'));

        /** @var array<string, string> $links */
        $links = config('filesystems.links');
        self::assertNotContains(storage_path('app/editorial-media'), array_values($links));
    }

    public function test_authorized_editor_uploads_reencoded_jpeg_without_metadata_and_with_thumbnail(): void
    {
        $actor = $this->createIdentity('media-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $marker = 'OTERYN-SECRET-EXIF-MARKER';
        $jpeg = $this->withJpegMetadata($this->imageBytes('jpeg', 400, 200), $marker);

        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('unsafe-original-name.jpeg', $jpeg, 'image/jpeg'),
            'alt_text' => 'A stone bridge leading into Oteryn.',
        ])->assertRedirect(route('admin.media.index'));

        $media = EditorialMedia::query()->firstOrFail();
        self::assertSame('image/jpeg', $media->mime_type);
        self::assertSame('jpg', $media->extension);
        self::assertSame(400, $media->width);
        self::assertSame(200, $media->height);
        self::assertSame(100, $media->thumbnail_width);
        self::assertSame(50, $media->thumbnail_height);
        self::assertMatchesRegularExpression('#^originals/[0-9a-f]{2}/[0-9a-f]{48}\.jpg$#', $media->storage_path);
        self::assertStringNotContainsString('unsafe-original-name', $media->storage_path);

        $stored = Storage::disk('editorial_media')->get($media->storage_path);
        self::assertSame(hash('sha256', $stored), $media->sha256);
        self::assertStringNotContainsString($marker, $stored);
        $decoded = getimagesizefromstring($stored);
        self::assertIsArray($decoded);
        self::assertSame(400, $decoded[0]);
        self::assertSame(200, $decoded[1]);
        self::assertSame('image/jpeg', $decoded['mime']);

        $thumbnailPath = $media->thumbnail_path;
        self::assertNotNull($thumbnailPath);
        $thumbnail = Storage::disk('editorial_media')->get($thumbnailPath);
        self::assertSame(hash('sha256', $thumbnail), $media->thumbnail_sha256);
        self::assertSame('private', Storage::disk('editorial_media')->getVisibility($media->storage_path));
        self::assertSame('private', Storage::disk('editorial_media')->getVisibility($thumbnailPath));

        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'editorial_media.uploaded',
            'target_type' => 'editorial_media',
            'target_id' => (string) $media->id,
        ]);
        $auditMetadata = DB::table('admin_audit_events')
            ->where('action', 'editorial_media.uploaded')
            ->where('target_id', (string) $media->id)
            ->value('metadata');
        self::assertIsString($auditMetadata);
        self::assertStringNotContainsString('A stone bridge leading into Oteryn.', $auditMetadata);
        self::assertStringNotContainsString('unsafe-original-name.jpeg', $auditMetadata);
        self::assertStringNotContainsString($marker, $auditMetadata);

        $this->get(route('admin.media.content', $media))
            ->assertOk()
            ->assertHeader('Content-Type', 'image/jpeg')
            ->assertHeader('Cache-Control', 'private, no-store')
            ->assertHeader('X-Content-Type-Options', 'nosniff');
    }

    public function test_png_metadata_is_removed_and_small_png_and_webp_do_not_create_thumbnails(): void
    {
        $actor = $this->createIdentity('media-raster-editor@example.com');
        $marker = 'OTERYN-PNG-METADATA-MARKER';
        $png = $this->withPngTextMetadata($this->imageBytes('png', 40, 30), $marker);

        $pngMedia = app(StoreEditorialImage::class)->execute(
            $actor,
            $this->rawUpload('metadata.png', $png, 'image/png'),
            'PNG metadata removal fixture.',
        );
        $webpMedia = app(StoreEditorialImage::class)->execute(
            $actor,
            $this->rawUpload('small.webp', $this->imageBytes('webp', 40, 30), 'image/webp'),
            'Small WebP fixture.',
        );

        self::assertStringNotContainsString(
            $marker,
            Storage::disk('editorial_media')->get($pngMedia->storage_path),
        );
        self::assertNull($pngMedia->thumbnail_path);
        self::assertNull($webpMedia->thumbnail_path);
        self::assertSame('image/webp', $webpMedia->mime_type);
    }

    public function test_malicious_malformed_mismatched_and_over_limit_files_are_rejected(): void
    {
        $actor = $this->createIdentity('media-fixture-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $fixtures = [
            ['vector.svg', '<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>'],
            ['page.html', '<!doctype html><script>alert(1)</script>'],
            ['script.jpg', '<?php echo "executed";'],
            ['archive.png', "PK\x03\x04not-an-image"],
            ['program.webp', "MZ\x90\x00not-an-image"],
            ['document.png', "%PDF-1.7\nnot-an-image"],
            ['malformed.png', random_bytes(128)],
            ['polyglot.jpg', $this->imageBytes('jpeg', 20, 20).'<?php echo "executed";'],
            ['polyglot.png', $this->imageBytes('png', 20, 20).'<script>alert(1)</script>'],
            ['mismatch.jpg', $this->imageBytes('png', 20, 20)],
        ];

        foreach ($fixtures as [$name, $bytes]) {
            $this->post(route('admin.media.store'), [
                'image' => $this->rawUpload($name, $bytes),
                'alt_text' => 'Rejected fixture.',
            ])->assertSessionHasErrors('image');
        }

        config(['editorial_media.max_bytes' => 16]);
        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('too-large.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => 'Too large.',
        ])->assertSessionHasErrors('image');

        config([
            'editorial_media.max_bytes' => 8 * 1024 * 1024,
            'editorial_media.max_width' => 10,
        ]);
        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('too-wide.png', $this->imageBytes('png', 20, 5), 'image/png'),
            'alt_text' => 'Too wide.',
        ])->assertSessionHasErrors('image');

        config([
            'editorial_media.max_width' => 5000,
            'editorial_media.max_pixels' => 100,
        ]);
        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('too-many-pixels.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => 'Too many pixels.',
        ])->assertSessionHasErrors('image');

        $this->assertDatabaseCount('editorial_media', 0);
        self::assertSame([], Storage::disk('editorial_media')->allFiles());
    }

    public function test_alt_text_integrity_fields_and_storage_names_are_bounded_and_immutable(): void
    {
        $actor = $this->createIdentity('media-immutable-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('missing-alt.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => '',
        ])->assertSessionHasErrors('alt_text');
        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('long-alt.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => str_repeat('a', 501),
        ])->assertSessionHasErrors('alt_text');

        $media = $this->uploadThroughAction($actor, 'immutable.png');
        $originalPath = $media->storage_path;
        $media->storage_path = 'originals/changed.png';

        try {
            $media->save();
            self::fail('Editorial media storage paths must remain immutable.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('immutable', $exception->getMessage());
        }

        try {
            $media->refresh()->delete();
            self::fail('Direct model deletion must be rejected.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('safe deletion action', $exception->getMessage());
        }

        self::assertSame($originalPath, $media->fresh()?->storage_path);
        Storage::disk('editorial_media')->assertExists($originalPath);
    }

    public function test_exact_permission_confirmed_mfa_and_csrf_are_required(): void
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

        $editor = $this->createIdentity('media-csrf-editor@example.com');
        $this->assignRole($editor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($editor);
        $this->app->detectEnvironment(static fn (): string => 'production');
        $this->withMiddleware(ValidateCsrfToken::class);
        $this->post(route('admin.media.store'), [
            'image' => $this->rawUpload('csrf.png', $this->imageBytes('png', 20, 20), 'image/png'),
            'alt_text' => 'CSRF-protected upload.',
        ])->assertStatus(419);

        $this->assertDatabaseCount('editorial_media', 0);
    }

    public function test_referenced_media_cannot_be_deleted_and_unreferenced_media_is_deleted_with_audit(): void
    {
        $actor = $this->createIdentity('media-reference-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $referenced = $this->uploadThroughAction($actor, 'referenced.png');
        $reference = app(EditorialMediaReferenceManager::class)->attach(
            $referenced,
            EditorialMediaConsumer::WIKI,
            'article:42',
            'hero',
        );

        $this->delete(route('admin.media.destroy', $referenced))->assertSessionHasErrors('media');
        $this->assertDatabaseHas('editorial_media', ['id' => $referenced->id]);
        $this->assertDatabaseHas('editorial_media_references', ['id' => $reference->id]);

        try {
            DB::table('editorial_media')->where('id', $referenced->id)->delete();
            self::fail('The database must reject deletion while a reference exists.');
        } catch (\Throwable) {
            $this->assertDatabaseHas('editorial_media', ['id' => $referenced->id]);
        }

        $unreferenced = $this->uploadThroughAction($actor, 'delete-me.png', 300, 200);
        $thumbnailPath = $unreferenced->thumbnail_path;
        self::assertNotNull($thumbnailPath);
        $this->delete(route('admin.media.destroy', $unreferenced))
            ->assertRedirect(route('admin.media.index'));
        $this->assertDatabaseMissing('editorial_media', ['id' => $unreferenced->id]);
        Storage::disk('editorial_media')->assertMissing($unreferenced->storage_path);
        Storage::disk('editorial_media')->assertMissing($thumbnailPath);
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'editorial_media.deleted',
            'target_type' => 'editorial_media',
            'target_id' => (string) $unreferenced->id,
        ]);
    }

    public function test_storage_failures_leave_database_and_audit_state_unchanged(): void
    {
        $actor = $this->createIdentity('media-storage-editor@example.com');

        Storage::shouldReceive('disk')
            ->once()
            ->with('editorial_media')
            ->andThrow(new RuntimeException('storage unavailable'));

        try {
            app(StoreEditorialImage::class)->execute(
                $actor,
                $this->rawUpload('storage-down.png', $this->imageBytes('png', 20, 20), 'image/png'),
                'Storage failure fixture.',
            );
            self::fail('Storage failure must abort upload persistence.');
        } catch (RuntimeException $exception) {
            self::assertSame('storage unavailable', $exception->getMessage());
        }
        $this->assertDatabaseCount('editorial_media', 0);

        Storage::fake('editorial_media');
        $media = $this->uploadThroughAction($actor, 'missing-on-delete.png');
        Storage::disk('editorial_media')->delete($media->storage_path);

        try {
            app(DeleteEditorialImage::class)->execute($actor, $media);
            self::fail('Missing storage must abort record deletion.');
        } catch (RuntimeException $exception) {
            self::assertStringContainsString('deletion was refused', $exception->getMessage());
        }

        $this->assertDatabaseHas('editorial_media', ['id' => $media->id]);
        $this->assertDatabaseMissing('admin_audit_events', [
            'action' => 'editorial_media.deleted',
            'target_id' => (string) $media->id,
        ]);
    }

    private function uploadThroughAction(
        Identity $actor,
        string $name,
        int $width = 40,
        int $height = 30,
    ): EditorialMedia {
        return app(StoreEditorialImage::class)->execute(
            $actor,
            $this->rawUpload($name, $this->imageBytes('png', $width, $height), 'image/png'),
            'Editorial media action fixture.',
        );
    }

    private function createIdentity(string $email, bool $confirmedMfa = true): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        if ($confirmedMfa) {
            $identity->forceFill([
                'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
                'two_factor_confirmed_at' => now(),
            ])->save();
        }

        return $identity;
    }

    private function assignRole(Identity $identity, string $roleKey): void
    {
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');

        if (! is_int($roleId) && ! (is_string($roleId) && ctype_digit($roleId))) {
            self::fail('Expected an integer-compatible administrator role id.');
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => (int) $roleId,
        ]);
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }

    private function rawUpload(string $name, string $bytes, string $mimeType = 'application/octet-stream'): UploadedFile
    {
        $path = tempnam(sys_get_temp_dir(), 'oteryn-media-');

        if (! is_string($path) || file_put_contents($path, $bytes) === false) {
            self::fail('Could not create an editorial media test fixture.');
        }

        return new UploadedFile($path, $name, $mimeType, null, true);
    }

    private function imageBytes(string $format, int $width, int $height): string
    {
        $image = imagecreatetruecolor($width, $height);
        self::assertInstanceOf(GdImage::class, $image);
        $background = imagecolorallocate($image, 30, 70, 110);
        self::assertIsInt($background);
        self::assertTrue(imagefill($image, 0, 0, $background));
        ob_start();

        try {
            $encoded = match ($format) {
                'jpeg' => imagejpeg($image, null, 90),
                'png' => imagepng($image, null, 6),
                'webp' => imagewebp($image, null, 85),
                default => false,
            };
            $bytes = ob_get_contents();
        } finally {
            ob_end_clean();
            imagedestroy($image);
        }

        self::assertTrue($encoded);
        self::assertIsString($bytes);

        return $bytes;
    }

    private function withPngTextMetadata(string $png, string $marker): string
    {
        $iend = "\x00\x00\x00\x00IEND\xAE\x42\x60\x82";
        self::assertStringEndsWith($iend, $png);
        $chunkType = 'tEXt';
        $chunkData = "Comment\x00".$marker;
        $chunk = pack('N', strlen($chunkData))
            .$chunkType
            .$chunkData
            .pack('N', crc32($chunkType.$chunkData));

        return substr($png, 0, -strlen($iend)).$chunk.$iend;
    }

    private function withJpegMetadata(string $jpeg, string $marker): string
    {
        self::assertStringStartsWith("\xFF\xD8", $jpeg);
        $payload = "Exif\x00\x00".$marker;
        $segment = "\xFF\xE1".pack('n', strlen($payload) + 2).$payload;

        return substr($jpeg, 0, 2).$segment.substr($jpeg, 2);
    }
}
