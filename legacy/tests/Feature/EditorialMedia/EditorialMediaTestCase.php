<?php

namespace Tests\Feature\EditorialMedia;

use App\EditorialMedia\Application\Actions\StoreEditorialImage;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use GdImage;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Http\UploadedFile;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Storage;
use Tests\TestCase;

abstract class EditorialMediaTestCase extends TestCase
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

    protected function uploadThroughAction(
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

    protected function createIdentity(string $email, bool $confirmedMfa = true): Identity
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

    protected function assignRole(Identity $identity, string $roleKey): void
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

    protected function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }

    protected function rawUpload(string $name, string $bytes, string $mimeType = 'application/octet-stream'): UploadedFile
    {
        $path = tempnam(sys_get_temp_dir(), 'oteryn-media-');

        if (! is_string($path) || file_put_contents($path, $bytes) === false) {
            self::fail('Could not create an editorial media test fixture.');
        }

        return new UploadedFile($path, $name, $mimeType, null, true);
    }

    protected function imageBytes(string $format, int $width, int $height): string
    {
        if ($width < 1 || $height < 1) {
            self::fail('Image fixture dimensions must be positive.');
        }

        $image = imagecreatetruecolor($width, $height);
        self::assertInstanceOf(GdImage::class, $image);
        $background = imagecolorallocate($image, 30, 70, 110);
        self::assertIsInt($background);
        imagefill($image, 0, 0, $background);
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

        return $bytes;
    }

    protected function storedBytes(string $path): string
    {
        $bytes = Storage::disk('editorial_media')->get($path);

        if (! is_string($bytes)) {
            self::fail('Expected stored editorial media bytes.');
        }

        return $bytes;
    }

    protected function withPngTextMetadata(string $png, string $marker): string
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

    protected function withJpegMetadata(string $jpeg, string $marker): string
    {
        self::assertStringStartsWith("\xFF\xD8", $jpeg);
        $payload = "Exif\x00\x00".$marker;
        $segment = "\xFF\xE1".pack('n', strlen($payload) + 2).$payload;

        return substr($jpeg, 0, 2).$segment.substr($jpeg, 2);
    }
}
