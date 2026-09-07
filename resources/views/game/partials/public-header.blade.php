@php
    // Presentation grouping only: every registered item is retained, including future items.
    $navigationGroups = ['world' => [], 'chronicles' => [], 'knowledge' => [], 'explore' => []];
    $directItems = [];
    foreach ($headerItems as $item) {
        $key = match (true) {
            str_starts_with($item['active'], 'game.') => 'world',
            str_starts_with($item['active'], 'today.'), str_starts_with($item['active'], 'news.'), str_starts_with($item['active'], 'events.') => 'chronicles',
            str_starts_with($item['active'], 'wiki.'), str_starts_with($item['active'], 'game-catalog.'), str_starts_with($item['active'], 'editorial.') => 'knowledge',
            str_starts_with($item['active'], 'home'), str_starts_with($item['active'], 'downloads.'), str_starts_with($item['active'], 'support.') => null,
            default => 'explore',
        };
        if ($key === null) {
            $directItems[] = $item;
        } else {
            $navigationGroups[$key][] = $item;
        }
    }
@endphp
<header class="site-header">
    <div class="header-inner">
        <a class="brand portal-brand" href="{{ route('home') }}" aria-label="Oteryn Platform {{ __('public.navigation.home') }}">
            <img class="brand-wordmark-art" src="{{ asset('images/oteryn-wordmark.svg') }}" width="420" height="88" alt="" aria-hidden="true">
        </a>
        <nav class="primary-nav desktop-only" aria-label="{{ __('public.navigation.primary') }}">
            @foreach ($directItems as $item)
                @if (str_starts_with($item['active'], 'home'))
                    @include('game.partials.navigation-link', ['item' => $item, 'linkClass' => 'nav-link'])
                @endif
            @endforeach
            @foreach ($navigationGroups as $groupKey => $items)
                @if ($items !== [])
                    <details class="nav-group" name="portal-navigation">
                        <summary>{{ __('portal.navigation.'.$groupKey) }}</summary>
                        <div class="nav-group-links">
                            @foreach ($items as $item)
                                @include('game.partials.navigation-link', ['item' => $item, 'linkClass' => 'nav-link'])
                            @endforeach
                        </div>
                    </details>
                @endif
            @endforeach
            @foreach ($directItems as $item)
                @unless (str_starts_with($item['active'], 'home'))
                    @include('game.partials.navigation-link', ['item' => $item, 'linkClass' => 'nav-link'])
                @endunless
            @endforeach
        </nav>
        <div class="account-actions desktop-only">
            @include('game.partials.language-switcher')
            @guest
                <a class="nav-link" href="{{ route('identity.login.create') }}">{{ __('public.account.sign_in') }}</a>
                <a class="button" href="{{ route('identity.register.create') }}">{{ __('public.account.create') }}</a>
            @else
                <a class="button" href="{{ route('account.overview') }}">{{ __('public.account.title') }}</a>
                <form method="POST" action="{{ route('identity.logout') }}">@csrf<button class="button-ghost" type="submit">{{ __('public.account.sign_out') }}</button></form>
            @endguest
        </div>
        <details class="mobile-nav">
            <summary>{{ __('public.navigation.menu') }}</summary>
            <div class="mobile-nav-panel">
                <nav aria-label="{{ __('public.navigation.mobile') }}">
                    <div class="mobile-nav-group">
                        @foreach ($directItems as $item)
                            @include('game.partials.navigation-link', ['item' => $item, 'linkClass' => 'nav-link'])
                        @endforeach
                    </div>
                    @foreach ($navigationGroups as $groupKey => $items)
                        @if ($items !== [])
                            <div class="mobile-nav-group">
                                <p class="eyebrow">{{ __('portal.navigation.'.$groupKey) }}</p>
                                @foreach ($items as $item)
                                    @include('game.partials.navigation-link', ['item' => $item, 'linkClass' => 'nav-link'])
                                @endforeach
                            </div>
                        @endif
                    @endforeach
                </nav>
                <div class="mobile-account-actions">
                    @guest
                        <a class="button" href="{{ route('identity.register.create') }}">{{ __('public.account.create') }}</a>
                        <a class="button button-secondary" href="{{ route('identity.login.create') }}">{{ __('public.account.sign_in') }}</a>
                    @else
                        <a class="button" href="{{ route('account.overview') }}">{{ __('public.account.overview') }}</a>
                        <form method="POST" action="{{ route('identity.logout') }}">@csrf<button class="button-secondary" type="submit">{{ __('public.account.sign_out') }}</button></form>
                    @endguest
                    @include('game.partials.language-switcher')
                </div>
            </div>
        </details>
    </div>
</header>
