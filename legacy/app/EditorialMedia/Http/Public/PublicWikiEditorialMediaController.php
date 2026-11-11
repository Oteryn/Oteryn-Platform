<?php

namespace App\EditorialMedia\Http\Public;

use App\EditorialMedia\Application\WikiEditorialMediaFileResponse;
use App\EditorialMedia\Application\WikiEditorialMediaUnavailable;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\Wiki\Application\Media\WikiMediaAccess;
use App\Wiki\Application\Media\WikiMediaSyntax;
use App\Wiki\Domain\WikiContentRules;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Symfony\Component\HttpFoundation\Response as HttpResponse;

final readonly class PublicWikiEditorialMediaController
{
    public function __construct(
        private WikiMediaAccess $access,
        private WikiEditorialMediaFileResponse $files,
    ) {}

    public function __invoke(Request $request): Response
    {
        $locale = $request->route('locale', 'en');
        $editorialMediaId = $request->route('editorialMedia');
        abort_unless(is_string($locale), HttpResponse::HTTP_NOT_FOUND);
        if (! is_string($editorialMediaId) || ! ctype_digit($editorialMediaId)) {
            return $this->notFound($locale);
        }

        $mediaId = WikiMediaSyntax::mediaId('wiki-media:'.$editorialMediaId);
        if ($mediaId === null) {
            return $this->notFound($locale);
        }

        WikiContentRules::assertSupportedLocale($locale);

        try {
            $editorialMedia = EditorialMedia::query()->find($mediaId);
            if (
                ! $editorialMedia instanceof EditorialMedia
                || ! $this->access->allowsPublic($editorialMedia, $locale)
            ) {
                return $this->notFound($locale);
            }

            $response = $this->files->public($request, $editorialMedia);
            $response->headers->set('Content-Language', $locale);

            return $response;
        } catch (QueryException|WikiEditorialMediaUnavailable $exception) {
            report($exception);

            return response('', HttpResponse::HTTP_SERVICE_UNAVAILABLE, [
                'Cache-Control' => 'private, no-store',
                'Content-Language' => $locale,
                'X-Content-Type-Options' => 'nosniff',
            ]);
        }
    }

    private function notFound(string $locale): Response
    {
        return response('', HttpResponse::HTTP_NOT_FOUND, [
            'Cache-Control' => 'no-store',
            'Content-Language' => $locale,
            'X-Content-Type-Options' => 'nosniff',
        ]);
    }
}
