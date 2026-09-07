@php
    $adminNavigationGroups = [
        'overview' => [
            ['admin.dashboard', 'dashboard', 'admin.dashboard'],
        ],
        'content' => [
            ['admin.news.index', 'news', 'admin.news.*'],
            ['admin.pages.index', 'pages', 'admin.pages.*'],
            ['admin.events.index', 'events', 'admin.events.*'],
            ['admin.announcements.index', 'announcements', 'admin.announcements.*'],
            ['admin.downloads.index', 'downloads', 'admin.downloads.*'],
            ['admin.media.index', 'media', 'admin.media.*'],
            ['admin.wiki.index', 'wiki', 'admin.wiki.*'],
            ['admin.support-content.index', 'editorial', 'admin.support-content.*'],
            ['admin.homepage-templates.index', 'homepage', 'admin.homepage-templates.*'],
        ],
        'access' => [['admin.roles.index', 'roles', 'admin.roles.*']],
        'support' => [
            ['admin.support.tickets.index', 'tickets', 'admin.support.tickets.*'],
            ['admin.moderation.reports.index', 'reports', 'admin.moderation.reports.*'],
            ['admin.moderation.enforcement.index', 'enforcement', 'admin.moderation.enforcement.*'],
        ],
        'operations' => [
            ['admin.game-catalog.index', 'catalog', 'admin.game-catalog.*'],
            ['admin.payments.reconciliation.index', 'payments', 'admin.payments.*'],
            ['admin.marketplace.index', 'bazaar', 'admin.marketplace.*'],
            ['admin.audit.index', 'audit', 'admin.audit.*'],
        ],
    ];
@endphp
@foreach ($adminNavigationGroups as $group => $links)
    <p class="admin-nav-group">{{ __('portal_art.admin.'.$group) }}</p>
    @foreach ($links as [$destination, $label, $activePattern])
        @if ($destination !== 'admin.marketplace.index' || config('marketplace.enabled'))
            @php
                $routeParameters = ['locale' => app()->getLocale()];
                $href = match ($destination) {
                    'admin.media.index' => route('admin.media.index', $routeParameters),
                    'admin.payments.reconciliation.index' => route('admin.payments.reconciliation.index', $routeParameters),
                    default => route($destination, $routeParameters),
                };
            @endphp
            <a href="{{ $href }}" @if(request()->routeIs($activePattern)) aria-current="page" @endif>{{ __('portal_art.admin.'.$label) }}</a>
        @endif
    @endforeach
@endforeach
