@php
    $contextLinks = [];
    if (request()->routeIs('game.*', 'legacy.game.*')) {
        $contextLinks = [
            ['route' => 'game.servers.index', 'active' => 'game.servers.*', 'label' => 'public.game.servers_title'],
            ['route' => 'game.online.index', 'active' => 'game.online.*', 'label' => 'public.game.online_title'],
            ['route' => 'game.highscores.index', 'active' => 'game.highscores.*', 'label' => 'public.game.highscores_title'],
            ['route' => 'game.guilds.index', 'active' => 'game.guilds.*', 'label' => 'community.guilds.title'],
            ['route' => 'game.deaths.index', 'active' => 'game.deaths.*', 'label' => 'community.deaths.title'],
        ];
    } elseif (request()->routeIs('news.*', 'events.*', 'today.*', 'legacy.news.*', 'legacy.events.*', 'legacy.today.*')) {
        $contextLinks = [
            ['route' => 'today.index', 'active' => 'today.*', 'label' => 'public.navigation.today'],
            ['route' => 'news.index', 'active' => 'news.*', 'label' => 'public.news.title'],
            ['route' => 'events.index', 'active' => 'events.*', 'label' => 'public.events.title'],
        ];
    } elseif (request()->routeIs('wiki.*', 'game-catalog.*', 'legacy.wiki.*', 'legacy.game-catalog.*')) {
        $contextLinks = [
            ['route' => 'wiki.index', 'active' => 'wiki.index', 'label' => 'public.wiki.title'],
            ['route' => 'wiki.search', 'active' => 'wiki.search', 'label' => 'public.wiki.search_title'],
            ['route' => 'game-catalog.items.index', 'active' => 'game-catalog.items.*', 'label' => 'game_catalog.items'],
            ['route' => 'game-catalog.creatures.index', 'active' => 'game-catalog.creatures.*', 'label' => 'game_catalog.creatures'],
        ];
    }
@endphp
@if ($contextLinks !== [])
    <nav class="portal-context-nav" aria-label="{{ __('portal.navigation.section') }}">
        @foreach ($contextLinks as $contextLink)
            <a href="{{ route($contextLink['route']) }}" @if(request()->routeIs($contextLink['active'], 'legacy.'.$contextLink['active'])) aria-current="page" @endif>{{ __($contextLink['label']) }}</a>
        @endforeach
    </nav>
@endif
