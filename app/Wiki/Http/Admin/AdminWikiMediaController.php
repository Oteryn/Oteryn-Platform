<?php

namespace App\Wiki\Http\Admin;

use App\EditorialMedia\Application\WikiEditorialMediaFileResponse;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\Wiki\Application\Media\WikiMediaAccess;
use App\Wiki\Application\Media\WikiMediaSyntax;
use App\Wiki\Domain\WikiContentRules;
use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiArticleTranslation;
use Illuminate\Database\Eloquent\Builder;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Validation\ValidationException;
use InvalidArgumentException;
use RuntimeException;
use Symfony\Component\HttpFoundation\Response as HttpResponse;

final readonly class AdminWikiMediaController
{
    public function __construct(
        private WikiEditorialMediaFileResponse $files,
        private WikiMediaAccess $access,
    ) {}

    public function index(Request $request): JsonResponse
    {
        $validated = $request->validate([
            'q' => ['nullable', 'string', 'max:100'],
            'page' => ['nullable', 'integer', 'min:1', 'max:10000'],
        ]);
        if (! is_array($validated)) {
            throw new RuntimeException('Expected Laravel validation to return an array.');
        }

        $queryText = trim(is_string($validated['q'] ?? null) ? $validated['q'] : '');
        $query = EditorialMedia::query();

        if ($queryText !== '') {
            $pattern = '%'.addcslashes($queryText, '\\%_').'%';
            $query->where(function (Builder $media) use ($queryText, $pattern): void {
                if (ctype_digit($queryText)) {
                    $media->whereKey((int) $queryText);
                }

                $media->orWhere('alt_text', 'like', $pattern)
                    ->orWhere('original_name', 'like', $pattern);
            });
        }

        $items = $query
            ->orderByDesc('created_at')
            ->orderByDesc('id')
            ->paginate(12)
            ->withQueryString();

        return response()->json([
            'items' => $items->getCollection()->map(function (EditorialMedia $media): array {
                try {
                    $token = WikiMediaSyntax::markdownToken($media->id, $media->alt_text);
                } catch (InvalidArgumentException $exception) {
                    throw ValidationException::withMessages(['media' => $exception->getMessage()]);
                }

                return [
                    'id' => $media->id,
                    'alt_text' => $media->alt_text,
                    'mime_type' => $media->mime_type,
                    'width' => $media->width,
                    'height' => $media->height,
                    'markdown' => $token,
                    'thumbnail_url' => route('admin.wiki.media.thumbnail', $media),
                ];
            })->values(),
            'next_page_url' => $items->nextPageUrl(),
        ])->header('Cache-Control', 'private, no-store');
    }

    public function thumbnail(EditorialMedia $editorialMedia): Response
    {
        return $this->files->private($editorialMedia, true);
    }

    public function preview(
        WikiArticle $article,
        string $locale,
        WikiArticleTranslation $translation,
        EditorialMedia $editorialMedia,
    ): Response {
        WikiContentRules::assertSupportedLocale($locale);
        abort_unless(
            $translation->article_id === $article->id && $translation->locale === $locale,
            HttpResponse::HTTP_NOT_FOUND,
        );
        abort_unless(
            $this->access->allowsPreview($translation, $editorialMedia),
            HttpResponse::HTTP_NOT_FOUND,
        );

        return $this->files->private($editorialMedia, false);
    }
}
