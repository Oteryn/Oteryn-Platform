@extends('game.layout')

@section('title', $result->page?->title ?? __('public.editorial.labels.'.$key->value))

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    @if ($result->state === \App\Cms\Editorial\EditorialPageState::Published)
        @php($page = $result->page)
        <article>
            <p class="eyebrow">{{ $key->isLegal() ? __('public.editorial.legal') : ($key->isSupportGuidance() ? __('public.editorial.support') : __('public.editorial.learn')) }}</p>
            <h1>{{ $page->title }}</h1>

            @if ($key->isLegal() && $page->legal_version !== null && $page->legal_effective_date !== null)
                <p class="muted">
                    {{ __('public.editorial.version_effective', [
                        'version' => $page->legal_version,
                        'date' => $localeFormatter->date($page->legal_effective_date),
                    ]) }}
                </p>
            @endif

            <div class="card">
                <p class="prose-text">{{ $page->body }}</p>
            </div>

            @if ($supportLinks !== [])
                <section aria-labelledby="approved-support-links">
                    <h2 id="approved-support-links">{{ __('public.editorial.approved_channels') }}</h2>
                    <div class="card-grid">
                        @foreach ($supportLinks as $link)
                            <article class="card">
                                <h3>{{ $link['label'] }}</h3>
                                @if ($link['detail'] !== null)
                                    <p class="muted">{{ $link['detail'] }}</p>
                                @endif
                                <a class="button button-secondary"
                                   href="{{ $link['href'] }}"
                                   @if ($link['external']) target="_blank" rel="noopener noreferrer" @endif>
                                    {{ __('public.editorial.open') }}
                                </a>
                            </article>
                        @endforeach
                    </div>
                </section>
            @endif

            @if ($key === \App\Cms\Editorial\EditorialPageKey::ReportABug)
                <div class="notice">
                    {{ __('public.editorial.report_warning') }}
                </div>
            @endif
        </article>
    @else
        <article>
            <p class="eyebrow">{{ __('public.editorial.content') }}</p>
            <h1>{{ __('public.editorial.labels.'.$key->value) }}</h1>

            <div class="empty-state">
                @if ($result->state === \App\Cms\Editorial\EditorialPageState::Missing)
                    <strong>{{ __('public.editorial.missing') }}</strong>
                    <p>{{ __('public.editorial.missing_help') }}</p>
                @elseif ($result->state === \App\Cms\Editorial\EditorialPageState::TranslationUnavailable)
                    <strong>{{ __('public.editorial.translation_unavailable') }}</strong>
                    <p>{{ __('public.editorial.translation_unavailable_help') }}</p>
                @else
                    <strong>{{ __('public.editorial.unpublished') }}</strong>
                    <p>{{ __('public.editorial.unpublished_help') }}</p>
                @endif
            </div>
        </article>
    @endif
@endsection
