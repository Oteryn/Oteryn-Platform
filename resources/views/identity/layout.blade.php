<!DOCTYPE html>
<html lang="{{ str_replace('_', '-', app()->getLocale()) }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="robots" content="noindex,nofollow,noarchive">
    <title>@yield('title') · {{ config('app.name') }}</title>
    <link rel="stylesheet" href="{{ asset('css/app.css') }}">
    <link rel="stylesheet" href="{{ asset('css/mfa.css') }}">
    <link rel="stylesheet" href="{{ asset('css/support.css') }}">
</head>
<body class="identity-body">
<a class="skip-link" href="#main-content">{{ __('identity.layout.skip_to_content') }}</a>
<header class="site-header">
    <div class="header-inner">
        <a class="brand" href="{{ route('home') }}" aria-label="{{ __('identity.layout.home_label') }}">
            <span class="brand-mark" aria-hidden="true">OT</span>
            <span class="brand-label">Oteryn Platform</span>
        </a>
        <div class="account-actions">
            <a class="nav-link" href="{{ route('home') }}">{{ __('identity.layout.public_site') }}</a>
            @guest
                @unless(request()->routeIs('identity.login.*') || request()->routeIs('identity.mfa.challenge.*'))
                    <a class="nav-link" href="{{ route('identity.login.create') }}">{{ __('identity.layout.sign_in') }}</a>
                @endunless
                @unless(request()->routeIs('identity.register.*') || request()->routeIs('identity.mfa.challenge.*'))
                    <a class="button button-secondary" href="{{ route('identity.register.create') }}">{{ __('identity.layout.create_account') }}</a>
                @endunless
            @else
                <a class="nav-link" href="{{ route('account.overview') }}" @if(request()->routeIs('account.overview')) aria-current="page" @endif>{{ __('identity.layout.account') }}</a>
                <form method="POST" action="{{ route('identity.logout') }}">
                    @csrf
                    <button class="button-ghost" type="submit">{{ __('identity.layout.sign_out') }}</button>
                </form>
            @endguest
        </div>
    </div>
</header>

@auth
<div class="context-nav-wrap">
    <nav class="context-nav" aria-label="{{ __('identity.layout.account_actions') }}">
        <a href="{{ route('account.overview') }}" @if(request()->routeIs('account.overview')) aria-current="page" @endif>{{ __('identity.layout.overview') }}</a>
        <a href="{{ route('payments.account.index') }}" @if(request()->routeIs('payments.account.*')) aria-current="page" @endif>{{ __('payments.nav.account') }}</a>
        <a href="{{ route('identity.account-security.show') }}" @if(request()->routeIs('identity.account-security.*') || request()->routeIs('identity.sessions.*') || request()->routeIs('identity.email-change.*') || request()->routeIs('identity.privacy.*') || request()->routeIs('identity.recovery-key.generate') || request()->routeIs('identity.recovery-key.revoke') || request()->routeIs('identity.termination.*')) aria-current="page" @endif>{{ __('identity.layout.account_security') }}</a>
        <a href="{{ route('support.tickets.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('support.tickets.*') || request()->routeIs('support.reports.*') || request()->routeIs('support.enforcement.*')) aria-current="page" @endif>{{ __('support.nav.support_center') }}</a>
        <a href="{{ route('identity.mfa.settings') }}" @if(request()->routeIs('identity.mfa.settings')) aria-current="page" @endif>{{ __('identity.layout.authenticator') }}</a>
        <a href="{{ route('identity.password.change.create') }}" @if(request()->routeIs('identity.password.change.*')) aria-current="page" @endif>{{ __('identity.layout.password') }}</a>
    </nav>
</div>
@endauth

<main id="main-content" class="identity-shell">
    <section class="identity-panel">
        @if (session('status'))
            <div class="alert alert-success" role="status">{{ session('status') }}</div>
        @endif

        @if ($errors->any())
            <div class="alert alert-danger" role="alert">
                <p><strong>@yield('error-title', __('identity.layout.request_failed'))</strong></p>
                <ul>
                    @foreach ($errors->all() as $error)
                        <li>{{ $error }}</li>
                    @endforeach
                </ul>
            </div>
        @endif

        @yield('content')
    </section>
</main>
</body>
</html>
