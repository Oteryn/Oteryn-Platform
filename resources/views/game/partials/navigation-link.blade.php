@php
    $activePatterns = explode('|', $item['active']);
    $legacyPatterns = array_map(static fn (string $pattern): string => 'legacy.'.$pattern, $activePatterns);
    $active = request()->routeIs(...array_merge($activePatterns, $legacyPatterns))
        || ($item['active'] === 'home' && request()->routeIs('localized.home'));
@endphp
<a @if($linkClass ?? null) class="{{ $linkClass }}" @endif href="{{ $item['url'] }}" @if($active) aria-current="page" @endif>{{ $item['label'] }}</a>
