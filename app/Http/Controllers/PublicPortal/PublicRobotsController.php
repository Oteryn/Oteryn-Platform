<?php

namespace App\Http\Controllers\PublicPortal;

use Illuminate\Http\Request;
use Illuminate\Http\Response;

final class PublicRobotsController
{
    public function __invoke(Request $request): Response
    {
        $lines = [
            'User-agent: *',
            'Disallow: /admin',
            'Disallow: /account',
            'Disallow: /login',
            'Disallow: /register',
            'Disallow: /forgot-password',
            'Disallow: /reset-password',
            'Disallow: /password',
            'Disallow: /mfa',
            'Disallow: /oauth',
            'Disallow: /design',
            'Disallow: /wiki/search',
            'Disallow: /en/wiki/search',
            'Disallow: /pl/wiki/search',
            'Sitemap: '.route('seo.sitemap'),
            '',
        ];
        $content = implode("\n", $lines);
        $response = response($content)
            ->header('Content-Type', 'text/plain; charset=UTF-8')
            ->header('Cache-Control', 'public, no-cache, must-revalidate');

        $response->setEtag(hash('sha256', $content));
        $response->isNotModified($request);

        return $response;
    }
}
