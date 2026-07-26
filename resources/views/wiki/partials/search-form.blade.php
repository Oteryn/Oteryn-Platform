<form class="wiki-search-form" method="GET" action="{{ route('wiki.search') }}" role="search">
    <label for="wiki-search">{{ __('public.wiki.search_label') }}</label>
    <div class="wiki-search-controls">
        <input
            id="wiki-search"
            name="q"
            type="search"
            value="{{ $query ?? '' }}"
            minlength="2"
            maxlength="80"
            placeholder="{{ __('public.wiki.search_placeholder') }}"
            aria-describedby="wiki-search-help"
        >
        <button class="button" type="submit">{{ __('public.wiki.search_action') }}</button>
    </div>
    <p id="wiki-search-help" class="muted">{{ __('public.wiki.search_help') }}</p>
</form>
