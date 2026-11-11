<nav class="context-nav" aria-label="{{ __('support.nav.support_center') }}">
    <a href="{{ route('support.tickets.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('support.tickets.*')) aria-current="page" @endif>{{ __('support.nav.tickets') }}</a>
    <a href="{{ route('support.reports.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('support.reports.*')) aria-current="page" @endif>{{ __('support.nav.reports') }}</a>
    <a href="{{ route('support.enforcement.index', ['locale' => app()->getLocale()]) }}" @if(request()->routeIs('support.enforcement.*')) aria-current="page" @endif>{{ __('support.nav.enforcement') }}</a>
</nav>
