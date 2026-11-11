@php
    $localizedUrls ??= app(\App\Localization\LocalizedUrlGenerator::class)->forRequest(request());
    $publicLocales = app(\App\Localization\PublicLocale::class)->supported();
@endphp
<nav class="language-switcher" aria-label="{{ __('public.language.label') }}">
    @foreach ($publicLocales as $publicLocale)
        @php
            $languageName = trans('public.language_name', [], $publicLocale);
            $localeUrl = $localizedUrls->alternates[$publicLocale] ?? null;
            $active = app()->getLocale() === $publicLocale;
        @endphp
        @if ($localeUrl !== null)
            <a class="language-switcher__item{{ $active ? ' is-active' : '' }}"
               data-locale="{{ $publicLocale }}"
               href="{{ $localeUrl }}"
               lang="{{ $publicLocale }}"
               hreflang="{{ $publicLocale }}"
               @if ($active) aria-current="page" @else aria-label="{{ __('public.language.switch_to', ['language' => $languageName]) }}" @endif>
                {{ strtoupper($publicLocale) }}
            </a>
        @else
            <span class="language-switcher__item is-disabled"
                  data-locale="{{ $publicLocale }}"
                  lang="{{ $publicLocale }}"
                  aria-disabled="true"
                  title="{{ __('public.language.unavailable', ['language' => $languageName]) }}">
                {{ strtoupper($publicLocale) }}
            </span>
        @endif
    @endforeach
</nav>
