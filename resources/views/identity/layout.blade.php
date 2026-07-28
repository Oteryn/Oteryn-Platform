<!DOCTYPE html>
<html lang="{{ str_replace('_', '-', app()->getLocale()) }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="robots" content="noindex,nofollow,noarchive">
    <title>@yield('title') · {{ config('app.name') }}</title>
    <link rel="stylesheet" href="{{ asset('css/app.css') }}">
    <link rel="stylesheet" href="{{ asset('css/mfa.css') }}">
</head>
<body class="identity-body">
<a class="skip-link" href="#main-content">Skip to content</a>
<header class="site-header">
    <div class="header-inner">
        <a class="brand" href="{{ route('home') }}" aria-label="Oteryn Platform home">
            <span class="brand-mark" aria-hidden="true">OT</span>
            <span class="brand-label">Oteryn Platform</span>
        </a>
        <div class="account-actions">
            <a class="nav-link" href="{{ route('home') }}">Public site</a>
            @guest
                @unless(request()->routeIs('identity.login.*') || request()->routeIs('identity.mfa.challenge.*'))
                    <a class="nav-link" href="{{ route('identity.login.create') }}">Sign in</a>
                @endunless
                @unless(request()->routeIs('identity.register.*') || request()->routeIs('identity.mfa.challenge.*'))
                    <a class="button button-secondary" href="{{ route('identity.register.create') }}">Create account</a>
                @endunless
            @else
                <a class="nav-link" href="{{ route('account.overview') }}" @if(request()->routeIs('account.overview')) aria-current="page" @endif>Account</a>
                <form method="POST" action="{{ route('identity.logout') }}">
                    @csrf
                    <button class="button-ghost" type="submit">Sign out</button>
                </form>
            @endguest
        </div>
    </div>
</header>

@auth
<div class="context-nav-wrap">
    <nav class="context-nav" aria-label="Account actions">
        <a href="{{ route('account.overview') }}" @if(request()->routeIs('account.overview')) aria-current="page" @endif>Overview</a>
        <a href="{{ route('identity.account-security.show') }}" @if(request()->routeIs('identity.account-security.*') || request()->routeIs('identity.sessions.*') || request()->routeIs('identity.email-change.*') || request()->routeIs('identity.privacy.*') || request()->routeIs('identity.recovery-key.generate') || request()->routeIs('identity.recovery-key.revoke') || request()->routeIs('identity.termination.*')) aria-current="page" @endif>Account security</a>
        <a href="{{ route('identity.mfa.settings') }}" @if(request()->routeIs('identity.mfa.settings')) aria-current="page" @endif>Authenticator</a>
        <a href="{{ route('identity.password.change.create') }}" @if(request()->routeIs('identity.password.change.*')) aria-current="page" @endif>Password</a>
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
                <p><strong>@yield('error-title', 'The request could not be completed.')</strong></p>
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
