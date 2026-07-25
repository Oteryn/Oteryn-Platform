<footer class="site-footer">
    <div class="site-footer-inner public-footer-grid">
        <div class="public-footer-brand">
            <img class="brand-wordmark-art" src="{{ asset('images/oteryn-wordmark.svg') }}" alt="Oteryn Platform">
            <p>{{ __('public.footer.description') }}</p>
            <p class="public-footer-status-note">{{ __('public.footer.status') }}</p>
            @include('game.partials.language-switcher')
        </div>

        @foreach ($footerGroups as $group)
            <nav class="public-footer-group" aria-label="{{ __('public.navigation.group_links', ['group' => $group['label']]) }}">
                <h2>{{ $group['label'] }}</h2>
                @foreach ($group['items'] as $item)
                    @php($active = request()->routeIs($item['active']) || request()->routeIs('legacy.'.$item['active']))
                    <a href="{{ $item['url'] }}" @if($active) aria-current="page" @endif>{{ $item['label'] }}</a>
                @endforeach
            </nav>
        @endforeach

        <nav class="public-footer-group" aria-label="{{ __('public.account.links') }}">
            <h2>{{ __('public.account.title') }}</h2>
            @guest
                <a href="{{ route('identity.login.create') }}">{{ __('public.account.sign_in') }}</a>
                <a href="{{ route('identity.register.create') }}">{{ __('public.account.create') }}</a>
                <a href="{{ route('password.request') }}">{{ __('public.account.recover_password') }}</a>
            @else
                <a href="{{ route('account.overview') }}">{{ __('public.account.overview') }}</a>
                <a href="{{ route('identity.mfa.settings') }}">{{ __('public.account.security') }}</a>
                <a href="{{ route('identity.password.change.create') }}">{{ __('public.account.change_password') }}</a>
            @endguest
        </nav>
    </div>

    <div class="public-footer-meta">
        <div>
            <span>&copy; {{ now()->year }} Oteryn Platform</span>
            <span>{{ __('public.language.current', ['language' => trans('public.language_name')]) }}</span>
        </div>
    </div>
</footer>
