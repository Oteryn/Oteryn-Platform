<nav class="action-row" aria-label="{{ __('identity.common.language') }}">
    <a class="button button-secondary" href="{{ route($localeRoute, array_merge($localeParameters ?? [], ['locale' => 'en'])) }}" @if(app()->getLocale() === 'en') aria-current="page" @endif>
        {{ __('identity.common.english') }}
    </a>
    <a class="button button-secondary" href="{{ route($localeRoute, array_merge($localeParameters ?? [], ['locale' => 'pl'])) }}" @if(app()->getLocale() === 'pl') aria-current="page" @endif>
        {{ __('identity.common.polish') }}
    </a>
</nav>
