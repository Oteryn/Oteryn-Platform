@inject('adminAuthorization', 'App\Admin\AdminAuthorization')
@php
    // Presentation only. Route middleware remains the authorization authority.
    $adminNavigationGroups = [
        'overview' => [['admin.dashboard', 'dashboard', 'admin.dashboard', 'admin.access']],
        'content' => [
            ['admin.news.index', 'news', 'admin.news.*', 'cms.news.manage'],
            ['admin.pages.index', 'pages', 'admin.pages.*', 'cms.pages.manage'],
            ['admin.events.index', 'events', 'admin.events.*', 'events.manage'],
            ['admin.announcements.index', 'announcements', 'admin.announcements.*', 'portal.announcements.manage'],
            ['admin.downloads.index', 'downloads', 'admin.downloads.*', 'downloads.manage'],
            ['admin.media.index', 'media', 'admin.media.*', 'media.manage'],
            ['admin.wiki.index', 'wiki', 'admin.wiki.*', 'wiki.access'],
            ['admin.support-content.index', 'editorial', 'admin.support-content.*', 'support.content.manage'],
            ['admin.homepage-templates.index', 'homepage', 'admin.homepage-templates.*', 'portal.settings.manage'],
        ],
        'support' => [
            ['admin.support.tickets.index', 'tickets', 'admin.support.tickets.*', 'support.tickets.manage'],
            ['admin.moderation.reports.index', 'reports', 'admin.moderation.reports.*', 'support.reports.manage'],
            ['admin.moderation.enforcement.index', 'enforcement', 'admin.moderation.enforcement.*', 'support.enforcement.manage'],
        ],
        'access' => [['admin.roles.index', 'roles', 'admin.roles.*', 'admin.roles.manage']],
        'operations' => [
            ['admin.game-catalog.index', 'catalog', 'admin.game-catalog.*', 'game_catalog.access'],
            ['admin.payments.reconciliation.index', 'payments', 'admin.payments.*', 'payments.reconcile'],
            ['admin.marketplace.index', 'bazaar', 'admin.marketplace.*', 'marketplace.manage'],
            ['admin.audit.index', 'audit', 'admin.audit.*', 'audit.view'],
        ],
    ];
    $adminIdentity = auth()->user();
    $grantedPermissions = $adminIdentity instanceof \App\Identity\Models\Identity
        ? $adminAuthorization->grantedPermissions($adminIdentity)
        : [];
@endphp
@foreach ($adminNavigationGroups as $group => $links)
    @php
        $visibleLinks = array_filter($links, static fn ($link) => in_array($link[3], $grantedPermissions, true)
            && ($link[0] !== 'admin.marketplace.index' || config('marketplace.enabled')));
    @endphp
    @continue($visibleLinks === [] || (($asDashboard ?? false) && $group === 'overview'))
    @if ($asDashboard ?? false)
        <section class="admin-task-group admin-task-group-{{ $group }}" aria-labelledby="admin-task-{{ $group }}">
            <h2 id="admin-task-{{ $group }}">{{ __('portal_art.admin.'.$group) }}</h2>
            <p class="muted">{{ __('portal_art.admin.'.$group.'_help') }}</p>
            <div class="admin-task-links">
    @else
        <p class="admin-nav-group">{{ __('portal_art.admin.'.$group) }}</p>
    @endif
    @foreach ($visibleLinks as [$destination, $label, $activePattern, $permission])
        @php
            $routeParameters = ['locale' => app()->getLocale()];
            $href = match ($destination) {
                'admin.media.index' => route('admin.media.index', $routeParameters),
                'admin.payments.reconciliation.index' => route('admin.payments.reconciliation.index', $routeParameters),
                'admin.audit.index' => route('admin.audit.index', $routeParameters),
                'admin.homepage-templates.index' => route('admin.homepage-templates.index', $routeParameters),
                'admin.roles.index' => route('admin.roles.index', $routeParameters),
                default => route($destination, $routeParameters),
            };
        @endphp
        <a href="{{ $href }}" @if(request()->routeIs($activePattern)) aria-current="page" @endif>
            {{ __('portal_art.admin.'.$label) }}
            @if ($asDashboard ?? false)<span aria-hidden="true">→</span>@endif
        </a>
    @endforeach
    @if ($asDashboard ?? false)
            </div>
        </section>
    @endif
@endforeach
