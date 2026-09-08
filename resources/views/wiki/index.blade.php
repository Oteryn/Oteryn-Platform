@extends('game.layout')

@section('title', __('public.wiki.title'))
@section('description', __('public.wiki.description'))
@section('page-class', 'wiki-page')

@section('portal-family', 'knowledge')

@section('content')
    <header class="page-header wiki-hero">
        <p class="eyebrow">{{ __('public.wiki.eyebrow') }}</p>
        <h1>{{ __('public.wiki.title') }}</h1>
        <p class="muted">{{ __('public.wiki.description') }}</p>
        @include('wiki.partials.search-form', ['query' => ''])
    </header>

    @if ($wiki->isEmpty())
        <div class="empty-state" role="status">
            <strong>{{ __('public.wiki.empty') }}</strong>
            <p>{{ __('public.wiki.empty_help') }}</p>
        </div>
    @else
        @php
            $featuredSlugs = array_map(static fn ($article) => $article->slug, $wiki->featuredArticles);
            $recentArticles = array_filter($wiki->recentArticles, static fn ($article) => ! in_array($article->slug, $featuredSlugs, true));
        @endphp
        <div class="knowledge-library">
        @if ($wiki->categories !== [])
            <section aria-labelledby="wiki-categories">
                <div class="section-heading">
                    <p class="eyebrow">{{ __('public.wiki.explore') }}</p>
                    <h2 id="wiki-categories">{{ __('public.wiki.categories') }}</h2>
                </div>
                <div class="card-grid wiki-category-grid">
                    @foreach ($wiki->categories as $category)
                        <article class="card">
                            <h3><a href="{{ route('wiki.category', ['slug' => $category->slug]) }}">{{ $category->name }}</a></h3>
                            @if ($category->description !== null && $category->description !== '')
                                <p>{{ $category->description }}</p>
                            @endif
                            <p class="muted">{{ trans_choice('public.wiki.article_count', $category->articleCount, ['count' => $category->articleCount]) }}</p>
                        </article>
                    @endforeach
                </div>
            </section>
        @endif

        <div class="knowledge-stream">
        @if ($wiki->featuredArticles !== [])
            <section aria-labelledby="wiki-featured">
                <div class="section-heading">
                    <p class="eyebrow">{{ __('public.wiki.featured') }}</p>
                    <h2 id="wiki-featured">{{ __('public.wiki.featured_articles') }}</h2>
                </div>
                <div class="card-grid">
                    @foreach ($wiki->featuredArticles as $articleCard)
                        @include('wiki.partials.article-card', ['articleCard' => $articleCard])
                    @endforeach
                </div>
            </section>
        @endif

        @if ($recentArticles !== [])
            <section aria-labelledby="wiki-recent">
                <div class="section-heading">
                    <p class="eyebrow">{{ __('public.wiki.recent') }}</p>
                    <h2 id="wiki-recent">{{ __('public.wiki.recent_articles') }}</h2>
                </div>
                <div class="card-grid">
                    @foreach ($recentArticles as $articleCard)
                        @include('wiki.partials.article-card', ['articleCard' => $articleCard])
                    @endforeach
                </div>
            </section>
        @endif
        </div>
        </div>
    @endif
@endsection
