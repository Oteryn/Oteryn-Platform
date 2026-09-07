<!DOCTYPE html>
<html lang="{{ str_replace('_', '-', app()->getLocale()) }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="robots" content="noindex,nofollow,noarchive">
    <title>@yield('title') · {{ config('app.name') }}</title>
    <link rel="stylesheet" href="{{ asset('css/portal-system.css') }}">
    <link rel="stylesheet" href="{{ asset('css/portal-pages.css') }}">
    @stack('head')
    @stack('styles')
</head>
<body class="identity-body @auth identity-authenticated @else identity-guest @endauth" data-portal-family="@yield('portal-family', 'identity')">
<a class="skip-link" href="#main-content">{{ __('identity.layout.skip_to_content') }}</a>
<header class="site-header">
    <div class="header-inner">
        <a class="brand" href="{{ route('home') }}" aria-label="{{ __('identity.layout.home_label') }}">
            <img class="brand-wordmark-art" src="{{ asset('images/oteryn-wordmark.svg') }}" width="420" height="88" alt="Oteryn Platform">
        </a>
        <div class="account-actions">
            <nav class="language-switcher" aria-label="{{ __('identity.common.language') }}">
                @foreach (['en' => 'EN', 'pl' => 'PL'] as $localeCode => $localeLabel)
                    <a class="language-switcher__item @if(app()->getLocale() === $localeCode) is-active @endif" href="{{ request()->fullUrlWithQuery(['locale' => $localeCode]) }}" lang="{{ $localeCode }}" @if(app()->getLocale() === $localeCode) aria-current="page" @endif>{{ $localeLabel }}</a>
                @endforeach
            </nav>
            <a class="nav-link" href="{{ route('home') }}">{{ __('identity.layout.public_site') }}</a>
            @guest
                @unless(request()->routeIs('identity.login.*') || request()->routeIs('identity.mfa.challenge.*'))
                    <a class="nav-link" href="{{ route('identity.login.create', ['locale' => app()->getLocale()]) }}">{{ __('identity.layout.sign_in') }}</a>
                @endunless
                @unless(request()->routeIs('identity.register.*') || request()->routeIs('identity.mfa.challenge.*'))
                    <a class="button button-secondary" href="{{ route('identity.register.create', ['locale' => app()->getLocale()]) }}">{{ __('identity.layout.create_account') }}</a>
                @endunless
            @else
                <form method="POST" action="{{ route('identity.logout') }}">
                    @csrf
                    <button class="button-ghost" type="submit">{{ __('identity.layout.sign_out') }}</button>
                </form>
            @endguest
        </div>
    </div>
</header>
<div class="identity-workspace">
    @auth
        <aside class="account-sidebar">
            <p class="eyebrow">{{ __('portal.account.workspace') }}</p>
            <nav class="context-nav" aria-label="{{ __('identity.layout.account_actions') }}">
        <a href="{{ route('account.overview', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('account.overview')) aria-current="page" @endif>{{ __('identity.layout.overview') }}</a>
        <a href="{{ route('account.overview', ['locale' => app()->getLocale()]) }}#account-characters-heading" @if(request()->routeIs('account.characters.*')) aria-current="page" @endif>{{ __('portal.account.characters') }}</a>
        <a href="{{ route('player-companion.session-analyses.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('player-companion.*')) aria-current="page" @endif>{{ __('portal.account.tools') }}</a>
        @if (config('marketplace.enabled'))
            <a href="{{ route('marketplace.account', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('marketplace.*')) aria-current="page" @endif>{{ __('marketplace.my_bazaar') }}</a>
        @endif
        <a href="{{ route('payments.account.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('payments.account.*')) aria-current="page" @endif>{{ __('payments.nav.account') }}</a>
        <a href="{{ route('identity.account-security.show', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('identity.account-security.*') || request()->routeIs('identity.sessions.*') || request()->routeIs('identity.email-change.*') || request()->routeIs('identity.privacy.*') || request()->routeIs('identity.recovery-key.generate') || request()->routeIs('identity.recovery-key.revoke') || request()->routeIs('identity.termination.*')) aria-current="page" @endif>{{ __('identity.layout.account_security') }}</a>
        <a href="{{ route('support.tickets.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('support.tickets.*') || request()->routeIs('support.reports.*') || request()->routeIs('support.enforcement.*')) aria-current="page" @endif>{{ __('support.nav.support_center') }}</a>
        <a href="{{ route('identity.mfa.settings', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('identity.mfa.settings')) aria-current="page" @endif>{{ __('identity.layout.authenticator') }}</a>
        <a href="{{ route('identity.password.change.create', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('identity.password.change.*')) aria-current="page" @endif>{{ __('identity.layout.password') }}</a>
    </nav>
        </aside>
    @else
        <aside class="identity-world" aria-hidden="true">
            <img src="{{ asset('images/oteryn-world.svg') }}" width="2400" height="1200" alt="">
            <p class="identity-world-title">OTERYN</p>
            <p class="identity-world-copy">{{ __('portal.account.world_copy') }}</p>
        </aside>
    @endauth
    <main id="main-content" class="identity-shell">
        <div class="identity-panel">
            @if (session('status'))
                <div class="alert alert-success" role="status">{{ session('status') }}</div>
            @endif
            @if ($errors->any())
                <div class="alert alert-danger" role="alert">
                    <p><strong>@yield('error-title', __('identity.layout.request_failed'))</strong></p>
                    <ul>
                        @foreach ($errors->getMessages() as $field => $messages)
                            <li id="error-{{ \Illuminate\Support\Str::slug($field) }}">{{ implode(' ', $messages) }}</li>
                        @endforeach
                    </ul>
                </div>
            @endif
            @yield('content')
        </div>
    </main>
</div>
<footer class="identity-footer">&copy; {{ now()->year }} Oteryn · <a href="{{ route('home') }}">{{ __('identity.layout.public_site') }}</a></footer>
</body>
</html>
