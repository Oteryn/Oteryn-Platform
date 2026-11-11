<!DOCTYPE html>
<html lang="{{ str_replace('_', '-', app()->getLocale()) }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="robots" content="noindex, nofollow">
    <title>@yield('title') · {{ config('app.name') }}</title>
    <link rel="stylesheet" href="{{ asset('css/portal-system.css') }}?v={{ substr(hash_file('sha256', public_path('css/portal-system.css')), 0, 12) }}">
    <link rel="stylesheet" href="{{ asset('css/portal-art-direction.css') }}?v={{ substr(hash_file('sha256', public_path('css/portal-art-direction.css')), 0, 12) }}">
</head>
<body class="error-body portal-error-body">
<main id="main-content" class="error-panel">
    <a class="brand" href="{{ route('home') }}" aria-label="Oteryn Platform {{ __('public.navigation.home') }}">
        <img class="brand-wordmark-art" src="{{ asset('images/oteryn-wordmark.svg') }}" width="420" height="88" alt="Oteryn Platform">
    </a>
    <div class="page-header">
        <p class="error-code">@yield('code')</p>
        <h1>@yield('heading')</h1>
        <p class="muted">@yield('message')</p>
    </div>
    <div class="action-row">
        @yield('actions')
    </div>
</main>
</body>
</html>
