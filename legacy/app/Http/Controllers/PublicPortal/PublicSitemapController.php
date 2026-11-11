<?php

namespace App\Http\Controllers\PublicPortal;

use App\PublicPortal\Seo\PublicSitemapQuery;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Throwable;

final class PublicSitemapController
{
    public function __invoke(Request $request, PublicSitemapQuery $sitemap): Response
    {
        try {
            $content = view('seo.sitemap', ['urls' => $sitemap->urls()])->render();
        } catch (Throwable $exception) {
            report($exception);

            return response('Sitemap temporarily unavailable.', 503)
                ->header('Content-Type', 'text/plain; charset=UTF-8')
                ->header('Cache-Control', 'no-store');
        }

        $response = response($content)
            ->header('Content-Type', 'application/xml; charset=UTF-8')
            ->header('Cache-Control', 'public, no-cache, must-revalidate');

        $response->setEtag(hash('sha256', $content));
        $response->isNotModified($request);

        return $response;
    }
}
