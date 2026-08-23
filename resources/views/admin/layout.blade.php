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
    @stack('head')
</head>
<body class="admin-body">
<a class="skip-link" href="#main-content">Skip to content</a>
<header class="admin-topbar">
    <div class="header-inner">
        <a class="brand" href="{{ route('admin.dashboard') }}" aria-label="Oteryn Admin dashboard">
            <span class="brand-mark" aria-hidden="true">OA</span>
            <span class="brand-label">Oteryn Admin</span>
        </a>
        <div class="account-actions">
            <a class="nav-link" href="{{ route('home') }}">Public site</a>
            <form method="POST" action="{{ route('identity.logout') }}">
                @csrf
                <button class="button-ghost" type="submit">Sign out</button>
            </form>
        </div>
    </div>
</header>

<div class="admin-shell">
    <aside class="admin-sidebar" aria-label="Administrator sections">
        <nav class="admin-nav">
            <p class="admin-nav-group">Overview</p>
            <a href="{{ route('admin.dashboard') }}" @if(request()->routeIs('admin.dashboard')) aria-current="page" @endif>Dashboard</a>
            <p class="admin-nav-group">Content</p>
            <a href="{{ route('admin.news.index') }}" @if(request()->routeIs('admin.news.*')) aria-current="page" @endif>News</a>
            <a href="{{ route('admin.pages.index') }}" @if(request()->routeIs('admin.pages.*')) aria-current="page" @endif>Managed pages</a>
            <a href="{{ route('admin.media.index') }}" @if(request()->routeIs('admin.media.*')) aria-current="page" @endif>Editorial media</a>
            <a href="{{ route('admin.wiki.index') }}" @if(request()->routeIs('admin.wiki.*')) aria-current="page" @endif>Wiki</a>
            <p class="admin-nav-group">Access</p>
            <a href="{{ route('admin.roles.index') }}" @if(request()->routeIs('admin.roles.*')) aria-current="page" @endif>Roles</a>
            <p class="admin-nav-group">Support</p>
            <a href="{{ route('admin.support.tickets.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('admin.support.tickets.*')) aria-current="page" @endif>{{ __('support.nav.admin_tickets') }}</a>
            <a href="{{ route('admin.moderation.reports.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('admin.moderation.reports.*')) aria-current="page" @endif>{{ __('support.nav.admin_reports') }}</a>
            <a href="{{ route('admin.moderation.enforcement.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('admin.moderation.enforcement.*')) aria-current="page" @endif>{{ __('support.nav.admin_enforcement') }}</a>
            <p class="admin-nav-group">Operations</p>
            <a href="{{ route('admin.game-catalog.index') }}" @if(request()->routeIs('admin.game-catalog.*')) aria-current="page" @endif>Game Catalog</a>
            <a href="{{ route('admin.payments.reconciliation.index') }}" @if(request()->routeIs('admin.payments.*')) aria-current="page" @endif>{{ __('payments.nav.admin') }}</a>
            @if (config('marketplace.enabled'))
                <a href="{{ route('admin.marketplace.index') }}" @if(request()->routeIs('admin.marketplace.*')) aria-current="page" @endif>Character Bazaar</a>
            @endif
            <a href="{{ route('admin.audit.index') }}" @if(request()->routeIs('admin.audit.*')) aria-current="page" @endif>Audit</a>
        </nav>
    </aside>

    <main id="main-content" class="admin-main">
        <details class="admin-mobile-nav">
            <summary>Administrator navigation</summary>
            <nav class="admin-nav" aria-label="Administrator navigation">
                <a href="{{ route('admin.dashboard') }}" @if(request()->routeIs('admin.dashboard')) aria-current="page" @endif>Dashboard</a>
                <a href="{{ route('admin.news.index') }}" @if(request()->routeIs('admin.news.*')) aria-current="page" @endif>News</a>
                <a href="{{ route('admin.pages.index') }}" @if(request()->routeIs('admin.pages.*')) aria-current="page" @endif>Managed pages</a>
                <a href="{{ route('admin.media.index') }}" @if(request()->routeIs('admin.media.*')) aria-current="page" @endif>Editorial media</a>
                <a href="{{ route('admin.wiki.index') }}" @if(request()->routeIs('admin.wiki.*')) aria-current="page" @endif>Wiki</a>
                <a href="{{ route('admin.roles.index') }}" @if(request()->routeIs('admin.roles.*')) aria-current="page" @endif>Roles</a>
                <a href="{{ route('admin.support.tickets.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('admin.support.tickets.*')) aria-current="page" @endif>{{ __('support.nav.admin_tickets') }}</a>
                <a href="{{ route('admin.moderation.reports.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('admin.moderation.reports.*')) aria-current="page" @endif>{{ __('support.nav.admin_reports') }}</a>
                <a href="{{ route('admin.moderation.enforcement.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('admin.moderation.enforcement.*')) aria-current="page" @endif>{{ __('support.nav.admin_enforcement') }}</a>
                <a href="{{ route('admin.game-catalog.index') }}" @if(request()->routeIs('admin.game-catalog.*')) aria-current="page" @endif>Game Catalog</a>
                <a href="{{ route('admin.payments.reconciliation.index') }}" @if(request()->routeIs('admin.payments.*')) aria-current="page" @endif>{{ __('payments.nav.admin') }}</a>
                @if (config('marketplace.enabled'))
                    <a href="{{ route('admin.marketplace.index') }}" @if(request()->routeIs('admin.marketplace.*')) aria-current="page" @endif>Character Bazaar</a>
                @endif
                <a href="{{ route('admin.audit.index') }}" @if(request()->routeIs('admin.audit.*')) aria-current="page" @endif>Audit</a>
            </nav>
        </details>

        @if (session('status'))
            <div class="alert alert-success" role="status">{{ session('status') }}</div>
        @endif

        @if ($errors->any())
            <div class="alert alert-danger" role="alert">
                <p><strong>The request could not be completed.</strong></p>
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
