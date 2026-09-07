<!DOCTYPE html>
<html lang="{{ str_replace('_', '-', app()->getLocale()) }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="robots" content="noindex,nofollow,noarchive">
    <title>@yield('title', 'Oteryn Admin') · {{ config('app.name') }}</title>
    <link rel="stylesheet" href="{{ asset('css/app.css') }}">
    <link rel="stylesheet" href="{{ asset('css/editorial-media-admin.css') }}">
    <link rel="stylesheet" href="{{ asset('css/wiki-admin.css') }}">
    <link rel="stylesheet" href="{{ asset('css/marketplace.css') }}">
    <link rel="stylesheet" href="{{ asset('css/marketplace-responsive.css') }}">
    <link rel="stylesheet" href="{{ asset('css/support.css') }}">
    <link rel="stylesheet" href="{{ asset('css/portal-art-direction.css') }}">
    <link rel="stylesheet" href="{{ asset('css/portal-admin.css') }}">
    @stack('head')
</head>
<body class="admin-body" data-portal-family="administration">
<a class="skip-link" href="#main-content">{{ __('public.skip_to_content') }}</a>
<header class="admin-topbar">
    <div class="header-inner">
        <a class="brand" href="{{ route('admin.dashboard') }}" aria-label="Oteryn Admin dashboard">
            <img class="brand-wordmark-art" src="{{ asset('images/oteryn-wordmark.svg') }}" width="420" height="88" alt="Oteryn">
            <span class="admin-brand-label">{{ __('portal_art.admin.console') }}</span>
        </a>
        <div class="account-actions">
            <a class="nav-link" href="{{ route('home') }}">{{ __('identity.layout.public_site') }}</a>
            <form method="POST" action="{{ route('identity.logout') }}">
                @csrf
                <button class="button-ghost" type="submit">{{ __('identity.layout.sign_out') }}</button>
            </form>
        </div>
    </div>
</header>

<div class="admin-shell">
    <aside class="admin-sidebar" aria-label="{{ __('portal_art.admin.navigation') }}">
        <nav class="admin-nav">
            @include('admin.partials.navigation')
        </nav>
    </aside>

    <main id="main-content" class="admin-main">
        <details class="admin-mobile-nav">
            <summary>{{ __('portal_art.admin.navigation') }}</summary>
            <nav class="admin-nav" aria-label="{{ __('portal_art.admin.navigation') }}">
                @include('admin.partials.navigation')
            </nav>
        </details>

        @if (session('status'))
            <div class="alert alert-success" role="status">{{ session('status') }}</div>
        @endif

        @if ($errors->any())
            <div class="alert alert-danger" role="alert">
                <p><strong>{{ __('portal_art.admin.request_error') }}</strong></p>
                <ul>
                    @foreach ($errors->all() as $error)
                        <li>{{ $error }}</li>
                    @endforeach
                </ul>
            </div>
        @endif

        @yield('content')
    </main>
</div>
</body>
</html>
