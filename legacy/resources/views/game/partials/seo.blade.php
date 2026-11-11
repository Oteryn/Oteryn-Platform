<meta name="description" content="{{ $description }}">
<meta name="robots" content="{{ $robots }}">
<meta property="og:site_name" content="{{ __('public.seo.site_name') }}">
<meta property="og:title" content="{{ $pageTitle }} · {{ config('app.name') }}">
<meta property="og:description" content="{{ $description }}">
<meta property="og:type" content="{{ $type }}">
<meta property="og:locale" content="{{ __('public.seo.og_locale') }}">
@php($seoAlternateLocale = app()->getLocale() === 'en' ? 'pl' : 'en')
@if (isset($localizedUrls->alternates[$seoAlternateLocale]))
    <meta property="og:locale:alternate" content="{{ __('public.seo.og_alternate_locale') }}">
@endif
@if ($localizedUrls->canonical !== null)
    <meta property="og:url" content="{{ $localizedUrls->canonical }}">
@endif
