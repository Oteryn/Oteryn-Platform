<!DOCTYPE html>
<html lang="{{ str_replace('_', '-', app()->getLocale()) }}">
<head>
    @php
        $seoPageTitle = \Illuminate\Support\Str::squish(strip_tags(html_entity_decode($__env->yieldContent('title'), ENT_QUOTES | ENT_HTML5, 'UTF-8')));
        $seoDescription = \Illuminate\Support\Str::squish(strip_tags(html_entity_decode($__env->yieldContent('description'), ENT_QUOTES | ENT_HTML5, 'UTF-8')));
        $seoDescription = $seoDescription !== ''
            ? \Illuminate\Support\Str::limit($seoDescription, 160, '')
            : __('public.seo.default_description', ['title' => $seoPageTitle]);
        $seoRobotsRequested = trim($__env->yieldContent('robots'));
        $seoRobots = in_array($seoRobotsRequested, ['noindex,follow', 'noindex,nofollow', 'noindex,nofollow,noarchive'], true)
            ? $seoRobotsRequested
            : 'index,follow';
        $seoType = trim($__env->yieldContent('og-type')) === 'article' ? 'article' : 'website';
        $localizedUrls = app(\App\Localization\LocalizedUrlGenerator::class)->forRequest(request());
        $portalAsset = static function (string $path): string {
            $absolute = public_path($path);
            $hash = is_file($absolute) ? hash_file('sha256', $absolute) : false;
            $version = is_string($hash) ? substr($hash, 0, 12) : 'missing';

            return asset($path).'?v='.$version;
        };
        $portalAssetRevision = substr(hash('sha256', implode('|', array_map(
            static function (string $path): string {
                $absolute = public_path($path);
                $hash = is_file($absolute) ? hash_file('sha256', $absolute) : false;

                return is_string($hash) ? $hash : 'missing';
            },
            ['css/portal-system.css', 'css/portal-pages.css', 'css/portal-art-direction.css', 'css/portal-owner-visible.css', 'js/portal-navigation.js'],
        ))), 0, 12);
    @endphp
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{{ $seoPageTitle }} · {{ config('app.name') }}</title>
    @include('game.partials.localized-seo', ['localizedUrls' => $localizedUrls])
    @include('game.partials.seo', [
        'pageTitle' => $seoPageTitle,
        'description' => $seoDescription,
        'robots' => $seoRobots,
        'type' => $seoType,
        'localizedUrls' => $localizedUrls,
    ])
    @stack('head')
    <link rel="stylesheet" href="{{ $portalAsset('css/portal-system.css') }}">
    <link rel="stylesheet" href="{{ $portalAsset('css/portal-pages.css') }}">
    <link rel="stylesheet" href="{{ $portalAsset('css/portal-art-direction.css') }}">
    <script src="{{ $portalAsset('js/portal-navigation.js') }}" defer></script>
    @stack('styles')
    <link rel="stylesheet" href="{{ $portalAsset('css/portal-owner-visible.css') }}">
</head>
<body class="public-body" data-portal-family="@yield('portal-family', 'public')" data-portal-assets="{{ $portalAssetRevision }}">
@inject('publicNavigation', 'App\PublicPortal\Navigation\PublicNavigationRegistry')
<a class="skip-link" href="#main-content">{{ __('public.skip_to_content') }}</a>
@include('game.partials.public-header', ['headerItems' => $publicNavigation->header()])
<main id="main-content" class="page-shell @yield('page-class')">
    @include('game.partials.page-context')
    @yield('content')
</main>
@include('game.partials.public-footer', ['footerGroups' => $publicNavigation->footer()])
</body>
</html>
