<!DOCTYPE html>
<html lang="{{ str_replace('_', '-', app()->getLocale()) }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="robots" content="noindex, nofollow">
    <title>@yield('title') · {{ config('app.name') }}</title>
    <link rel="stylesheet" href="{{ asset('css/app.css') }}">
</head>
<body class="error-body">
<main id="main-content" class="error-panel">
    <a class="brand" href="{{ route('home') }}" aria-label="Oteryn Platform {{ __('public.navigation.home') }}">
        <span class="brand-mark" aria-hidden="true">OT</span>
        <span>Oteryn Platform</span>
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
