@php
    $localizedUrls ??= app(\App\Localization\LocalizedUrlGenerator::class)->forRequest(request());
@endphp
@if ($localizedUrls->canonical !== null)
    <link rel="canonical" href="{{ $localizedUrls->canonical }}">
@endif
@foreach ($localizedUrls->alternates as $alternateLocale => $alternateUrl)
    <link rel="alternate" hreflang="{{ $alternateLocale }}" href="{{ $alternateUrl }}">
@endforeach
@if (isset($localizedUrls->alternates['en']))
    <link rel="alternate" hreflang="x-default" href="{{ $localizedUrls->alternates['en'] }}">
@endif
