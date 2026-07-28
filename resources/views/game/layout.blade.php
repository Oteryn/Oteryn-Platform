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
    <link rel="stylesheet" href="{{ asset('css/app.css') }}">
    <link rel="stylesheet" href="{{ asset('css/portal.css') }}">
    <link rel="stylesheet" href="{{ asset('css/brand-art.css') }}">
    <link rel="stylesheet" href="{{ asset('css/public-shell.css') }}">
    <link rel="stylesheet" href="{{ asset('css/marketplace.css') }}">
    @stack('styles')
</head>
<body class="public-body">
@inject('publicNavigation', 'App\PublicPortal\Navigation\PublicNavigationRegistry')
<a class="skip-link" href="#main-content">{{ __('public.skip_to_content') }}</a>
@include('game.partials.public-header', ['headerItems' => $publicNavigation->header()])
<main id="main-content" class="page-shell @yield('page-class')">
    @yield('content')
</main>
@include('game.partials.public-footer', ['footerGroups' => $publicNavigation->footer()])
</body>
</html>
