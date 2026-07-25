<header class="site-header">
    <div class="header-inner">
        <a class="brand portal-brand" href="{{ route('home') }}" aria-label="Oteryn Platform {{ __('public.navigation.home') }}">
            <img class="brand-wordmark-art" src="{{ asset('images/oteryn-wordmark.svg') }}" alt="" aria-hidden="true">
        </a>

        <nav class="primary-nav desktop-only" aria-label="{{ __('public.navigation.primary') }}">
            @foreach ($headerItems as $item)
                @php($active = request()->routeIs($item['active']) || request()->routeIs('legacy.'.$item['active']))
                <a class="nav-link" href="{{ $item['url'] }}" @if($active) aria-current="page" @endif>{{ $item['label'] }}</a>
            @endforeach
        </nav>

        <div class="account-actions desktop-only">
            @include('game.partials.language-switcher')
            @guest
                <a class="nav-link" href="{{ route('identity.login.create') }}" @if(request()->routeIs('identity.login.*')) aria-current="page" @endif>{{ __('public.account.sign_in') }}</a>
                <a class="button" href="{{ route('identity.register.create') }}">{{ __('public.account.create') }}</a>
            @else
                <a class="button button-secondary" href="{{ route('account.overview') }}">{{ __('public.account.center') }}</a>
                <form method="POST" action="{{ route('identity.logout') }}">
                    @csrf
                    <button class="button-ghost" type="submit">{{ __('public.account.sign_out') }}</button>
                </form>
            @endguest
        </div>

        <details class="mobile-nav">
            <summary aria-label="{{ __('public.navigation.open_menu') }}">{{ __('public.navigation.menu') }}</summary>
            <div class="mobile-nav-panel">
                <nav aria-label="{{ __('public.navigation.mobile') }}">
                    @foreach ($headerItems as $item)
                        @php($active = request()->routeIs($item['active']) || request()->routeIs('legacy.'.$item['active']))
                        <a class="nav-link" href="{{ $item['url'] }}" @if($active) aria-current="page" @endif>{{ $item['label'] }}</a>
                    @endforeach
                </nav>

                @include('game.partials.language-switcher')

                <div class="mobile-account-actions">
                    @guest
                        <a class="nav-link" href="{{ route('identity.login.create') }}">{{ __('public.account.sign_in') }}</a>
                        <a class="nav-link" href="{{ route('identity.register.create') }}">{{ __('public.account.create') }}</a>
                    @else
                        <a class="nav-link" href="{{ route('account.overview') }}">{{ __('public.account.overview') }}</a>
                        <a class="nav-link" href="{{ route('identity.mfa.settings') }}">{{ __('public.account.security') }}</a>
                        <a class="nav-link" href="{{ route('identity.password.change.create') }}">{{ __('public.account.change_password') }}</a>
                        <form method="POST" action="{{ route('identity.logout') }}">
                            @csrf
                            <button class="button-ghost" type="submit">{{ __('public.account.sign_out') }}</button>
                        </form>
                    @endguest
                </div>
            </div>
        </details>
    </div>
</header>
