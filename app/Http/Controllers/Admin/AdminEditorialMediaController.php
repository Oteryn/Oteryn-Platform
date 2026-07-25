<?php

namespace App\Http\Controllers\Admin;

use App\EditorialMedia\Application\Actions\DeleteEditorialImage;
use App\EditorialMedia\Application\Actions\StoreEditorialImage;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\Http\Requests\Admin\AdminEditorialMediaUploadRequest;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Http\UploadedFile;
use Illuminate\Support\Facades\Storage;
use RuntimeException;

final class AdminEditorialMediaController
{
    public function index(): View
    {
        return view('admin.media.index', [
            'mediaItems' => EditorialMedia::query()
                ->withCount('references')
                ->orderByDesc('created_at')
                ->orderByDesc('id')
                ->paginate(24),
        ]);
    }

    public function store(
        AdminEditorialMediaUploadRequest $request,
        StoreEditorialImage $store,
    ): RedirectResponse {
        $identity = $request->user();
        $uploadedFile = $request->file('image');

        abort_unless($identity instanceof Identity, 403);
        abort_unless($uploadedFile instanceof UploadedFile, 422);

        $store->execute(
            $identity,
            $uploadedFile,
            $request->string('alt_text')->toString(),
        );

        return redirect()
            ->route('admin.media.index')
            ->with('status', 'Editorial image uploaded safely.');
    }

    public function content(EditorialMedia $editorialMedia): Response
    {
        return $this->fileResponse($editorialMedia, false);
    }

    public function thumbnail(EditorialMedia $editorialMedia): Response
    {
        return $this->fileResponse($editorialMedia, true);
    }

    public function destroy(
        Request $request,
        EditorialMedia $editorialMedia,
        DeleteEditorialImage $delete,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $delete->execute($identity, $editorialMedia);

        return redirect()
            ->route('admin.media.index')
            ->with('status', 'Editorial image deleted.');
    }

    private function fileResponse(EditorialMedia $media, bool $thumbnail): Response
    {
        $path = $media->storage_path;
        $requiredPrefix = 'originals/';
        $expectedByteSize = $media->byte_size;
        $expectedSha256 = $media->sha256;

        if ($thumbnail && $media->thumbnail_path !== null) {
            $path = $media->thumbnail_path;
            $requiredPrefix = 'thumbnails/';
            $expectedByteSize = $media->thumbnail_byte_size;
            $expectedSha256 = $media->thumbnail_sha256;
        }

        if (! is_int($expectedByteSize) || ! is_string($expectedSha256)) {
            throw new RuntimeException('Editorial image integrity metadata is incomplete.');
        }

        if (! str_starts_with($path, $requiredPrefix) || str_contains($path, '..')) {
            throw new RuntimeException('Editorial image storage path is invalid.');
        }

        if ($media->disk !== 'editorial_media') {
            throw new RuntimeException('Editorial image storage disk is invalid.');
        }

        $filesystem = Storage::disk($media->disk);

        if (! $filesystem->exists($path)) {
            abort(404);
        }

        $bytes = $filesystem->get($path);

        if (! is_string($bytes)) {
            throw new RuntimeException('Editorial image storage could not be read safely.');
        }

        if (strlen($bytes) !== $expectedByteSize || ! hash_equals($expectedSha256, hash('sha256', $bytes))) {
            throw new RuntimeException('Editorial image integrity verification failed.');
        }

        return response($bytes, 200, [
            'Content-Type' => $media->mime_type,
            'Content-Length' => (string) strlen($bytes),
            'Content-Disposition' => 'inline; filename="'.basename($path).'"',
            'Cache-Control' => 'private, no-store',
            'X-Content-Type-Options' => 'nosniff',
        ]);
    }
}
